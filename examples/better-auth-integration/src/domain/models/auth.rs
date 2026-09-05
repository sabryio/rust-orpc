#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}
