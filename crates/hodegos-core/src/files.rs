use std::fs;
use std::io::Write;

use sha2::{Digest, Sha256};

use crate::project::Resolved;
use crate::{FileError, Language, Project, RelPath, Session};

/// Taille maximale d'un fichier ouvert dans l'éditeur.
pub const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

/// Version d'un fichier : empreinte de son contenu (SHA-256, 16 caractères hexadécimaux).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version(String);

impl Version {
    pub fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        Self(digest[..8].iter().map(|b| format!("{b:02x}")).collect())
    }

    /// Version transmise par le front, à comparer à celle du disque.
    pub fn from_client(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Fichier texte lu depuis le disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    pub path: RelPath,
    pub content: String,
    pub version: Version,
    pub language: Language,
}

impl Project {
    pub fn read_file(&self, session: &Session, rel: &RelPath) -> Result<TextFile, FileError> {
        self.check(session)?;
        let file = self.resolve_file(rel)?;
        let not_text = || FileError::NotText(rel.to_string());
        let size = fs::metadata(&file.real)
            .map_err(|e| FileError::io(rel.as_str(), e))?
            .len();
        if size > MAX_TEXT_BYTES {
            return Err(not_text());
        }
        let bytes = fs::read(&file.real).map_err(|e| FileError::io(rel.as_str(), e))?;
        let version = Version::of(&bytes);
        let content = String::from_utf8(bytes).map_err(|_| not_text())?;
        Ok(TextFile {
            path: rel.clone(),
            content,
            version,
            language: Language::of(rel.as_str()),
        })
    }

    /// Version actuelle sur le disque, `None` si le fichier n'existe plus.
    pub fn file_version(
        &self,
        session: &Session,
        rel: &RelPath,
    ) -> Result<Option<Version>, FileError> {
        self.check(session)?;
        let file = match self.resolve_file(rel) {
            Ok(file) => file,
            Err(FileError::NotFound(_)) => return Ok(None),
            Err(err) => return Err(err),
        };
        match fs::read(&file.real) {
            Ok(bytes) => Ok(Some(Version::of(&bytes))),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(FileError::io(rel.as_str(), err)),
        }
    }

    /// Écrit `content` si la version sur disque est encore `base`, sinon renvoie
    /// [`FileError::Conflict`]. L'écriture passe par un fichier temporaire renommé,
    /// et conserve les droits du fichier. Le fichier doit exister.
    pub fn write_file(
        &self,
        session: &Session,
        rel: &RelPath,
        content: &str,
        base: &Version,
    ) -> Result<Version, FileError> {
        self.check(session)?;
        if content.len() as u64 > MAX_TEXT_BYTES {
            return Err(FileError::NotText(rel.to_string()));
        }
        let io = |e| FileError::io(rel.as_str(), e);
        let file = self.resolve_file(rel)?;
        let current = Version::of(&fs::read(&file.real).map_err(io)?);
        if current != *base {
            return Err(FileError::Conflict {
                path: rel.to_string(),
                current,
            });
        }
        let permissions = fs::metadata(&file.real).map_err(io)?.permissions();
        let parent = file
            .real
            .parent()
            .ok_or_else(|| FileError::Forbidden(rel.to_string()))?;
        let mut temp = tempfile::Builder::new()
            .prefix(".hodegos-")
            .suffix(".tmp")
            .tempfile_in(parent)
            .map_err(io)?;
        temp.write_all(content.as_bytes()).map_err(io)?;
        temp.as_file().sync_all().map_err(io)?;
        fs::set_permissions(temp.path(), permissions).map_err(io)?;
        temp.persist(&file.real).map_err(|e| io(e.error))?;
        Ok(Version::of(content.as_bytes()))
    }

    fn resolve_file(&self, rel: &RelPath) -> Result<Resolved, FileError> {
        let file = self.resolve_existing(rel)?;
        if file.is_dir {
            return Err(FileError::NotFound(rel.to_string()));
        }
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::UserId;

    fn setup() -> (tempfile::TempDir, Project, Session) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join(".gitignore"), ".env\n").unwrap();
        fs::write(root.join(".env"), "SECRET=1").unwrap();
        fs::write(root.join("src/reserver.ts"), "const a = 1;\n").unwrap();
        fs::write(root.join("image.png"), [0x89, b'P', b'N', b'G', 0xff, 0xfe]).unwrap();
        let project = Project::open(root).unwrap();
        let session = project.session(UserId::local());
        (dir, project, session)
    }

    fn rel(raw: &str) -> RelPath {
        RelPath::parse(raw).unwrap()
    }

    #[test]
    fn lit_un_fichier_texte_avec_version_et_langage() {
        let (_dir, project, session) = setup();
        let file = project
            .read_file(&session, &rel("src/reserver.ts"))
            .unwrap();
        assert_eq!(file.content, "const a = 1;\n");
        assert_eq!(file.language, Language::TypeScript);
        assert_eq!(file.version, Version::of(b"const a = 1;\n"));
        assert_eq!(file.version.as_str().len(), 16);
    }

    #[test]
    fn refuse_un_fichier_binaire_ou_trop_gros() {
        let (dir, project, session) = setup();
        let err = project.read_file(&session, &rel("image.png")).unwrap_err();
        assert!(matches!(err, FileError::NotText(_)), "{err:?}");

        let big = vec![b'a'; MAX_TEXT_BYTES as usize + 1];
        fs::write(dir.path().join("gros.txt"), big).unwrap();
        let err = project.read_file(&session, &rel("gros.txt")).unwrap_err();
        assert!(matches!(err, FileError::NotText(_)), "{err:?}");
    }

    #[test]
    fn refuse_un_fichier_ignore_un_dossier_ou_un_absent() {
        let (_dir, project, session) = setup();
        let err = project.read_file(&session, &rel(".env")).unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        let err = project.read_file(&session, &rel("src")).unwrap_err();
        assert!(matches!(err, FileError::NotFound(_)), "{err:?}");
        let err = project.read_file(&session, &rel("absent.rs")).unwrap_err();
        assert!(matches!(err, FileError::NotFound(_)), "{err:?}");
    }

    #[test]
    fn refuse_un_lien_symbolique_vers_l_exterieur() {
        let (dir, project, session) = setup();
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("lien.txt")).unwrap();
        let err = project.read_file(&session, &rel("lien.txt")).unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
        let err = project
            .write_file(&session, &rel("lien.txt"), "x", &Version::of(b""))
            .unwrap_err();
        assert!(matches!(err, FileError::Forbidden(_)), "{err:?}");
    }

    #[test]
    fn ecrit_et_renvoie_la_nouvelle_version_en_gardant_les_droits() {
        let (dir, project, session) = setup();
        let path = dir.path().join("src/reserver.ts");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        let base = project
            .read_file(&session, &rel("src/reserver.ts"))
            .unwrap()
            .version;

        let version = project
            .write_file(&session, &rel("src/reserver.ts"), "const a = 2;\r\n", &base)
            .unwrap();
        assert_eq!(version, Version::of(b"const a = 2;\r\n"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "const a = 2;\r\n");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(
            project
                .file_version(&session, &rel("src/reserver.ts"))
                .unwrap(),
            Some(version)
        );
        let leftovers: Vec<_> = fs::read_dir(dir.path().join("src")).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "fichier temporaire resté sur le disque");
    }

    #[test]
    fn refuse_d_ecrire_sur_une_version_perimee() {
        let (dir, project, session) = setup();
        let base = project
            .read_file(&session, &rel("src/reserver.ts"))
            .unwrap()
            .version;
        fs::write(dir.path().join("src/reserver.ts"), "modifié ailleurs").unwrap();

        let err = project
            .write_file(&session, &rel("src/reserver.ts"), "à moi", &base)
            .unwrap_err();
        let FileError::Conflict { current, .. } = err else {
            panic!("conflit attendu : {err:?}");
        };
        assert_eq!(current, Version::of("modifié ailleurs".as_bytes()));
        assert_eq!(
            fs::read_to_string(dir.path().join("src/reserver.ts")).unwrap(),
            "modifié ailleurs"
        );
        project
            .write_file(&session, &rel("src/reserver.ts"), "à moi", &current)
            .unwrap();
    }

    #[test]
    fn la_version_d_un_fichier_supprime_est_absente() {
        let (dir, project, session) = setup();
        fs::remove_file(dir.path().join("src/reserver.ts")).unwrap();
        assert_eq!(
            project
                .file_version(&session, &rel("src/reserver.ts"))
                .unwrap(),
            None
        );
    }
}
