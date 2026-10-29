pub struct Inventaire {
    articles: Vec<String>,
}

impl Inventaire {
    pub fn plus_long(&self) -> String {
        let mut meilleur = self.articles[0].clone();
        for article in self.articles.clone() {
            if article.len() > meilleur.len() {
                meilleur = article.clone();
            }
        }
        meilleur
    }
}
