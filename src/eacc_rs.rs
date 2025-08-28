use alloy::sol;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    MarketPlaceData,
    "./src/abis/MarketplaceData.json"
);

pub struct EaccRs {
    pub name: String,
    pub version: String,
    pub marketplace_data: MarketPlaceData,
}
impl EaccRs {
    pub fn new(name: &str, version: &str) -> Self {
        EaccRs {
            name: name.to_string(),
            version: version.to_string(),
            marketplace_data: MarketPlaceData,
        }
    }
    pub fn display_info(&self) {
        println!("EaccRs Name: {}, Version: {}", self.name, self.version);
    }
}
