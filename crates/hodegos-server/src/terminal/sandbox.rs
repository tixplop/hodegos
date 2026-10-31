//! Confinement du shell par bubblewrap (ADR 0005, prototype du 31 octobre 2026) :
//! ligne de commande, montages de `$HOME`, environnement, et test de disponibilité.
//! Fonctions pures autant que possible, testées sans `bwrap`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

/// Montage d'un chemin de `$HOME` (ou absolu) dans le terminal.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Bind {
    /// `~/…` ou chemin absolu.
    pub path: String,
    #[serde(default)]
    pub mode: BindMode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BindMode {
    #[default]
    Ro,
    Rw,
}

impl Bind {
    fn new(path: &str, mode: BindMode) -> Self {
        Self {
            path: path.to_owned(),
            mode,
        }
    }
}

/// Montages par défaut : outils de développement et configuration du shell, en
/// lecture seule sauf les caches. `~/.ssh` n'y est jamais. Un chemin absent est ignoré.
pub fn default_binds() -> Vec<Bind> {
    use BindMode::{Ro, Rw};
    let mut binds = vec![
        Bind::new("~/.gitconfig", Ro),
        Bind::new("~/.cargo", Ro),
        // Caches de cargo seulement : rien n'est déposé dans ~/.cargo/bin.
        Bind::new("~/.cargo/registry", Rw),
        Bind::new("~/.cargo/git", Rw),
        Bind::new("~/.rustup", Ro),
        // Limite connue (ADR 0005) : ~/.npm/_npx contient du code que npx exécute.
        Bind::new("~/.npm", Rw),
        Bind::new("~/.nvm", Ro),
        Bind::new("~/.local/bin", Ro),
    ];
    for file in [
        ".zshrc",
        ".zprofile",
        ".zshenv",
        ".zlogin",
        ".bashrc",
        ".bash_profile",
        ".profile",
        ".inputrc",
        ".oh-my-zsh",
    ] {
        binds.push(Bind::new(&format!("~/{file}"), Ro));
    }
    binds
}

#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    terminal: RawTerminal,
}

#[derive(Deserialize, Default)]
struct RawTerminal {
    binds: Option<Vec<Bind>>,
}

/// Montages du terminal d'après `[terminal] binds` de `config.toml`, qui remplace la
/// liste par défaut ; un fichier ou une entrée invalide garde la liste par défaut et
/// renvoie un avertissement.
pub fn load_binds(config_dir: &Path) -> (Vec<Bind>, Option<String>) {
    let path = config_dir.join("config.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return (default_binds(), None);
    };
    let raw: RawConfig = match toml::from_str(&text) {
        Ok(raw) => raw,
        Err(err) => {
            return (
                default_binds(),
                Some(format!(
                    "{} : [terminal] illisible ({}), montages par défaut",
                    path.display(),
                    err.message()
                )),
            );
        }
    };
    match raw.terminal.binds {
        None => (default_binds(), None),
        Some(binds) => match binds
            .iter()
            .find(|b| !(b.path.starts_with("~/") || b.path.starts_with('/')))
        {
            Some(bad) => (
                default_binds(),
                Some(format!(
                    "[terminal] binds : « {} » doit commencer par ~/ ou /, montages par défaut",
                    bad.path
                )),
            ),
            None => (binds, None),
        },
    }
}

fn expand(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(path),
    }
}

/// Ce que voit le shell confiné.
pub struct Sandbox<'a> {
    pub project: &'a Path,
    pub home: &'a Path,
    pub binds: &'a [Bind],
    /// Cible de `/etc/resolv.conf` hors de `/etc` (WSL2 : `/mnt/wsl/resolv.conf`).
    pub resolv: Option<&'a Path>,
    pub shell: &'a str,
}

/// Arguments de `bwrap`, dans l'ordre : système en lecture seule, `/dev`, `/proc` et
/// `/tmp` neufs, `$HOME` vide puis ses montages (dans l'ordre de la liste : un cache
/// `rw` sous un dossier `ro`), projet en lecture-écriture, racine remise en lecture
/// seule. Pas de `--new-session` : il retire au shell son terminal de contrôle.
pub fn bwrap_args(sandbox: &Sandbox<'_>, exists: impl Fn(&Path) -> bool) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| args.extend(items.iter().map(|s| (*s).to_owned()));
    for dir in ["/usr", "/etc"] {
        push(&["--ro-bind", dir, dir]);
    }
    // Racines fusionnées dans /usr (liens) ou dossiers réels selon le système.
    for dir in ["/bin", "/sbin", "/lib", "/lib64", "/lib32"] {
        match fs::read_link(dir) {
            Ok(target) => push(&["--symlink", &target.to_string_lossy(), dir]),
            Err(_) if exists(Path::new(dir)) => push(&["--ro-bind", dir, dir]),
            Err(_) => {}
        }
    }
    if let Some(resolv) = sandbox.resolv {
        let resolv = resolv.to_string_lossy();
        push(&["--ro-bind", &resolv, &resolv]);
    }
    push(&["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"]);
    let home = sandbox.home.to_string_lossy();
    push(&["--dir", &home]);
    for bind in sandbox.binds {
        let path = expand(&bind.path, sandbox.home);
        if !exists(&path) {
            continue;
        }
        let path = path.to_string_lossy();
        let flag = match bind.mode {
            BindMode::Ro => "--ro-bind",
            BindMode::Rw => "--bind",
        };
        push(&[flag, &path, &path]);
    }
    let project = sandbox.project.to_string_lossy();
    push(&["--bind", &project, &project, "--chdir", &project]);
    push(&["--remount-ro", "/", "--unshare-pid", "--die-with-parent"]);
    push(&["--", sandbox.shell, "-i"]);
    args
}

/// Cible de `/etc/resolv.conf` quand c'est un lien hors de `/etc`.
pub fn resolv_target() -> Option<PathBuf> {
    fs::read_link("/etc/resolv.conf").ok()?;
    let target = fs::canonicalize("/etc/resolv.conf").ok()?;
    (!target.starts_with("/etc")).then_some(target)
}

/// Variables propres à Hodegos : jamais dans le terminal.
fn private(name: &str) -> bool {
    name == "OPENROUTER_API_KEY" || name.starts_with("HODEGOS_")
}

/// Environnement du shell : le vôtre, moins les variables de Hodegos ; historique et
/// caches dans le `/tmp` du terminal.
pub fn environment(vars: impl Iterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = vars.filter(|(name, _)| !private(name)).collect();
    let set = [
        ("TERM", "xterm-256color"),
        ("COLORTERM", "truecolor"),
        ("HISTFILE", "/tmp/.historique"),
        ("XDG_CACHE_HOME", "/tmp/.cache"),
        ("ZSH_CACHE_DIR", "/tmp/.cache/oh-my-zsh"),
    ];
    env.retain(|(name, _)| !set.iter().any(|(n, _)| n == name));
    env.extend(set.iter().map(|(n, v)| ((*n).to_owned(), (*v).to_owned())));
    env
}

/// Vérifie qu'un shell confiné peut démarrer : `bwrap` présent et espaces de noms
/// autorisés. Sinon, la raison et le remède.
pub fn probe(bwrap: &Path) -> Result<(), String> {
    let output = Command::new(bwrap)
        .args([
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--unshare-pid",
            "--die-with-parent",
            "--",
            "true",
        ])
        .stdin(Stdio::null())
        .output();
    match output {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(
            "bubblewrap (bwrap) est introuvable : installez-le avec « sudo apt install bubblewrap » ; sans lui, pas de terminal, jamais sans confinement".to_owned(),
        ),
        Err(err) => Err(format!("bwrap ne démarre pas : {err}")),
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            Err(format!(
                "bwrap refuse de créer les espaces de noms ({detail}) : vérifiez kernel.unprivileged_userns_clone et les restrictions d'AppArmor ; sans confinement, pas de terminal"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(binds: &[Bind], present: &[&str], resolv: Option<&Path>) -> Vec<String> {
        let present: Vec<PathBuf> = present.iter().map(PathBuf::from).collect();
        bwrap_args(
            &Sandbox {
                project: Path::new("/home/moi/projets/troc"),
                home: Path::new("/home/moi"),
                binds,
                resolv,
                shell: "/usr/bin/zsh",
            },
            |path| present.iter().any(|p| p == path),
        )
    }

    fn joined(args: &[String]) -> String {
        args.join(" ")
    }

    #[test]
    fn la_ligne_de_commande_suit_l_adr() {
        let all = args(
            &default_binds(),
            &[
                "/home/moi/.cargo",
                "/home/moi/.cargo/registry",
                "/home/moi/.zshrc",
                "/home/moi/.ssh",
            ],
            Some(Path::new("/mnt/wsl/resolv.conf")),
        );
        let line = joined(&all);
        assert!(
            line.starts_with("--ro-bind /usr /usr --ro-bind /etc /etc"),
            "{line}"
        );
        assert!(
            line.contains("--ro-bind /mnt/wsl/resolv.conf /mnt/wsl/resolv.conf"),
            "{line}"
        );
        assert!(
            line.contains("--proc /proc --dev /dev --tmpfs /tmp --dir /home/moi"),
            "{line}"
        );
        // Cache rw monté après le dossier ro qui le contient.
        let ro = line
            .find("--ro-bind /home/moi/.cargo /home/moi/.cargo")
            .unwrap();
        let rw = line
            .find("--bind /home/moi/.cargo/registry /home/moi/.cargo/registry")
            .unwrap();
        assert!(ro < rw, "{line}");
        assert!(
            line.contains("--ro-bind /home/moi/.zshrc /home/moi/.zshrc"),
            "{line}"
        );
        assert!(!line.contains(".ssh"), "jamais par défaut : {line}");
        assert!(!line.contains(".rustup"), "absent, ignoré : {line}");
        assert!(line.ends_with(
            "--bind /home/moi/projets/troc /home/moi/projets/troc --chdir /home/moi/projets/troc --remount-ro / --unshare-pid --die-with-parent -- /usr/bin/zsh -i"
        ), "{line}");
        assert!(!line.contains("--new-session"));
        assert!(
            !line.contains("--share-net") && !line.contains("--unshare-net"),
            "réseau conservé"
        );
    }

    #[test]
    fn une_liste_reglee_remplace_celle_par_defaut() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_binds(dir.path()).0, default_binds());
        fs::write(
            dir.path().join("config.toml"),
            "[llm]\nfast = \"a\"\n\n[terminal]\nbinds = [{ path = \"~/.ssh\", mode = \"ro\" }, { path = \"/opt/outils\", mode = \"rw\" }]\n",
        )
        .unwrap();
        let (binds, warning) = load_binds(dir.path());
        assert_eq!(warning, None);
        assert_eq!(
            binds,
            [
                Bind::new("~/.ssh", BindMode::Ro),
                Bind::new("/opt/outils", BindMode::Rw)
            ]
        );
        let line = joined(&args(&binds, &["/home/moi/.ssh", "/opt/outils"], None));
        assert!(
            line.contains("--ro-bind /home/moi/.ssh /home/moi/.ssh --bind /opt/outils /opt/outils"),
            "{line}"
        );

        fs::write(
            dir.path().join("config.toml"),
            "[terminal]\nbinds = [{ path = \"relatif\" }]\n",
        )
        .unwrap();
        let (binds, warning) = load_binds(dir.path());
        assert_eq!(binds, default_binds());
        assert!(warning.unwrap().contains("relatif"));
        fs::write(
            dir.path().join("config.toml"),
            "[terminal]\nbinds = [{ path = \"~/a\", mode = \"rwx\" }]\n",
        )
        .unwrap();
        assert!(load_binds(dir.path()).1.unwrap().contains("illisible"));
    }

    #[test]
    fn l_environnement_perd_les_variables_de_hodegos() {
        let vars = [
            ("PATH", "/usr/bin"),
            ("OPENROUTER_API_KEY", "sk-secret"),
            ("HODEGOS_LOG", "debug"),
            ("TERM", "dumb"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_owned(), b.to_owned()));
        let env = environment(vars);
        let get = |name: &str| env.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
        assert_eq!(get("PATH"), Some("/usr/bin"));
        assert_eq!(get("OPENROUTER_API_KEY"), None);
        assert_eq!(get("HODEGOS_LOG"), None);
        assert_eq!(get("TERM"), Some("xterm-256color"));
        assert_eq!(get("HISTFILE"), Some("/tmp/.historique"));
    }

    #[test]
    fn sans_bwrap_la_raison_et_le_remede() {
        let err = probe(Path::new("/chemin/absent/bwrap")).unwrap_err();
        assert!(err.contains("sudo apt install bubblewrap"), "{err}");
        assert!(err.contains("jamais sans confinement"), "{err}");
        let err = probe(Path::new("/bin/false")).unwrap_err();
        assert!(err.contains("espaces de noms"), "{err}");
    }
}
