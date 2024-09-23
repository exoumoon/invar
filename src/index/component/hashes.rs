use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Hashes {
    #[serde_as(as = "serde_with::hex::Hex")]
    sha1: [u8; 20],
    #[serde_as(as = "serde_with::hex::Hex")]
    sha512: [u8; 64],
}

#[cfg(test)]
mod tests {
    use super::Hashes;

    #[test]
    fn deserialize() {
        const JSON: &str = r#"{
            "sha1": "cc297357ff0031f805a744ca3a1378a112c2ddf4",
            "sha512": "d0760a2df6f123fb3546080a85f3a44608e1f8ad9f9f7c57b5380cf72235ad380a5bbd494263639032d63bb0f0c9e0847a62426a6028a73a4b4c8e7734b4e8f5"
        }"#;

        assert!(serde_json::from_str::<Hashes>(JSON).is_ok());
    }
}
