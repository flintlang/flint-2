#[derive(Debug, PartialEq, Clone, Default)]
pub struct Currency {
    pub identifier: &'static str,
    pub currency_types: Vec<&'static str>,
}
