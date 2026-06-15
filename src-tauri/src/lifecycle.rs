use tauri::AppHandle;
use async_trait::async_trait;

#[async_trait]
pub trait Lifecycle: Send + Sync {
    fn cache_key(&self) -> &'static str;

    async fn detect(&self, app: &AppHandle) -> Result<String, String>;

    async fn refresh(&self, app: &AppHandle) -> Result<String, String> {
        self.clear_cache();
        self.detect(app).await
    }

    fn clear_cache(&self) {
        println!("[Cache] cleared: {}", self.cache_key());
    }
}
