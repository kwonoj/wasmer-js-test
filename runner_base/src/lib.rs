
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SharedStruct {
    pub name: String,
    pub list: Vec<String>,
    pub other_list: Vec<u8>
}