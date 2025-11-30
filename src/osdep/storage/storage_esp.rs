use alloc::string::String;

pub async fn get_key(key: &str) -> Option<String> {
    None
}
pub async fn put_key(key: &str, value: &str) {}

pub fn get_key_sync(key: &str) -> Option<String> {
    todo!()
}
pub fn put_key_sync(key: &str, value: &str) {}
