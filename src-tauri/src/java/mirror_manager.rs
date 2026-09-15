use super::super::{logger, process_manager};
use super::super::types::MavenMirror;
use tauri::AppHandle;

/// 校验镜像 URL
pub fn validate_mirror_url(url: &str) -> Result<(), String> {
    process_manager::validate_mirror_url(url).map_err(|e| e.to_user_message())
}

/// 默认 Maven 镜像源列表
pub fn get_default_mirrors() -> Vec<MavenMirror> {
    vec![
        MavenMirror {
            name: "官方源".to_string(),
            url: "https://repo.maven.apache.org/maven2/".to_string(),
            active: true,
        },
        MavenMirror {
            name: "阿里云".to_string(),
            url: "https://maven.aliyun.com/repository/public/".to_string(),
            active: false,
        },
        MavenMirror {
            name: "华为云".to_string(),
            url: "https://repo.huaweicloud.com/repository/maven-public/".to_string(),
            active: false,
        },
        MavenMirror {
            name: "腾讯云".to_string(),
            url: "https://mirrors.cloud.tencent.com/nexus/repository/maven-public/".to_string(),
            active: false,
        },
    ]
}

/// 规范化 URL：去掉末尾斜杠便于比较
fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// 获取用户级 settings.xml 路径（~/.m2/settings.xml）
fn user_settings_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".m2").join("settings.xml"))
}

/// 读取当前 settings.xml 中配置的 mirror url
///
/// 简单解析：查找 `<mirrors>...</mirrors>` 块内的 `<url>...</url>`
/// Maven settings.xml 结构复杂，这里只取第一个 mirror 的 url 作为"当前激活的镜像"
fn read_current_mirror_url() -> Option<String> {
    let path = user_settings_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    parse_mirror_url_from_xml(&content)
}

/// 从 settings.xml 文本中解析第一个 mirror 的 url
fn parse_mirror_url_from_xml(content: &str) -> Option<String> {
    // 查找 <mirrors>...</mirrors> 块
    let mirrors_start = content.find("<mirrors>")?;
    let mirrors_end = content.find("</mirrors>")?;
    if mirrors_end <= mirrors_start {
        return None;
    }
    let mirrors_block = &content[mirrors_start..mirrors_end];

    // 在 mirrors 块中查找第一个 <url>...</url>
    let url_start = mirrors_block.find("<url>")?;
    let url_end = mirrors_block.find("</url>")?;
    let url = &mirrors_block[url_start + 5..url_end];
    Some(url.trim().to_string())
}

pub async fn list_maven_mirrors(app_handle: AppHandle) -> Result<Vec<MavenMirror>, String> {
    let mut mirrors = get_default_mirrors();

    match read_current_mirror_url() {
        Some(active_url) => {
            let active_normalized = normalize_url(&active_url);
            let mut found = false;
            for mirror in &mut mirrors {
                if normalize_url(&mirror.url) == active_normalized {
                    mirror.active = true;
                    found = true;
                } else {
                    mirror.active = false;
                }
            }
            // 若用户配置的 mirror 不在默认列表中，也标记所有为 inactive
            if !found {
                for m in &mut mirrors {
                    m.active = false;
                }
                logger::warn(
                    &app_handle,
                    &format!("当前 Maven 镜像不在默认列表中: {}", active_url),
                );
            }
        }
        None => {
            // settings.xml 不存在或无 mirrors 节点 → 实际使用官方源，但显示为未显式配置
            logger::info(
                &app_handle,
                "未发现用户级 settings.xml 或无 mirrors 配置，使用默认列表",
            );
        }
    }

    Ok(mirrors)
}

/// 生成 settings.xml 模板
fn build_settings_xml(mirror_name: &str, mirror_url: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0"
          xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
          xsi:schemaLocation="http://maven.apache.org/SETTINGS/1.0.0 https://maven.apache.org/xsd/settings-1.0.0.xsd">
  <mirrors>
    <mirror>
      <id>{name}</id>
      <mirrorOf>*</mirrorOf>
      <url>{url}</url>
    </mirror>
  </mirrors>
</settings>"#,
        name = mirror_name,
        url = mirror_url
    )
}

/// 替换 settings.xml 中的 <mirrors>...</mirrors> 块
fn replace_mirrors_block(content: &str, new_block: &str) -> String {
    if let (Some(start), Some(end)) = (content.find("<mirrors>"), content.find("</mirrors>")) {
        if end > start {
            let end_pos = end + "</mirrors>".len();
            return format!("{}{}{}", &content[..start], new_block, &content[end_pos..]);
        }
    }
    // 无 mirrors 块：在 </settings> 前插入
    if let Some(close) = content.find("</settings>") {
        return format!("{}  {}\n</settings>", &content[..close], new_block);
    }
    // 兜底：直接拼接
    format!("{}\n{}", content, new_block)
}

pub async fn switch_maven_mirror(
    app_handle: AppHandle,
    mirror_name: String,
    mirror_url: String,
) -> Result<String, String> {
    validate_mirror_url(&mirror_url)?;
    logger::info(
        &app_handle,
        &format!("正在切换 Maven 镜像到 {}...", mirror_name),
    );

    let settings_path = user_settings_path()
        .ok_or_else(|| "无法获取用户主目录".to_string())?;

    // 确保 ~/.m2 目录存在
    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建 .m2 目录失败: {}", e))?;
        }
    }

    // 读取现有 settings.xml（若存在）
    let existing_content = std::fs::read_to_string(&settings_path).ok();

    // 构造新的 mirrors 块
    let new_mirrors_block = format!(
        r#"<mirrors>
    <mirror>
      <id>{name}</id>
      <mirrorOf>*</mirrorOf>
      <url>{url}</url>
    </mirror>
  </mirrors>"#,
        name = mirror_name,
        url = mirror_url
    );

    let new_content = match existing_content {
        Some(content) => {
            // 备份原文件
            let backup_path = settings_path.with_extension("xml.bak");
            if let Err(e) = std::fs::copy(&settings_path, &backup_path) {
                logger::warn(&app_handle, &format!("备份 settings.xml 失败: {}", e));
            } else {
                logger::info(&app_handle, "已备份原 settings.xml 为 settings.xml.bak");
            }
            replace_mirrors_block(&content, &new_mirrors_block)
        }
        None => build_settings_xml(&mirror_name, &mirror_url),
    };

    std::fs::write(&settings_path, &new_content)
        .map_err(|e| format!("写入 settings.xml 失败: {}", e))?;

    logger::info(&app_handle, &format!("已切换到: {}", mirror_name));
    Ok(format!("已成功切换到 {}", mirror_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_mirror_url_rejects_empty() {
        assert!(validate_mirror_url("").is_err());
    }

    #[test]
    fn validate_mirror_url_rejects_invalid_scheme() {
        assert!(validate_mirror_url("ftp://mirror.example.com").is_err());
    }

    #[test]
    fn validate_mirror_url_accepts_valid() {
        assert!(validate_mirror_url("https://repo.maven.apache.org/maven2/").is_ok());
    }

    #[test]
    fn get_default_mirrors_has_four_entries() {
        let mirrors = get_default_mirrors();
        assert_eq!(mirrors.len(), 4);
        assert!(mirrors.iter().any(|m| m.name == "官方源"));
        assert!(mirrors.iter().any(|m| m.name == "阿里云"));
    }

    #[test]
    fn official_mirror_is_active_by_default() {
        let mirrors = get_default_mirrors();
        let official = mirrors.iter().find(|m| m.name == "官方源").unwrap();
        assert!(official.active);
    }

    #[test]
    fn normalize_url_strips_trailing_slash() {
        assert_eq!(normalize_url("https://example.com/"), "https://example.com");
        assert_eq!(normalize_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn parse_mirror_url_from_xml_extracts_url() {
        let xml = r#"<?xml version="1.0"?>
<settings>
  <mirrors>
    <mirror>
      <id>aliyun</id>
      <mirrorOf>*</mirrorOf>
      <url>https://maven.aliyun.com/repository/public/</url>
    </mirror>
  </mirrors>
</settings>"#;
        let url = parse_mirror_url_from_xml(xml).unwrap();
        assert_eq!(url, "https://maven.aliyun.com/repository/public/");
    }

    #[test]
    fn parse_mirror_url_from_xml_returns_none_without_mirrors() {
        let xml = r#"<?xml version="1.0"?>
<settings>
  <localRepository>/path</localRepository>
</settings>"#;
        assert!(parse_mirror_url_from_xml(xml).is_none());
    }

    #[test]
    fn replace_mirrors_block_replaces_existing() {
        let content = r#"<settings>
  <localRepository>/path</localRepository>
  <mirrors>
    <mirror>
      <id>old</id>
      <url>https://old.example.com</url>
    </mirror>
  </mirrors>
</settings>"#;
        let new_block = r#"<mirrors>
    <mirror>
      <id>new</id>
      <url>https://new.example.com</url>
    </mirror>
  </mirrors>"#;
        let result = replace_mirrors_block(content, new_block);
        assert!(result.contains("new.example.com"));
        assert!(!result.contains("old.example.com"));
    }

    #[test]
    fn replace_mirrors_block_inserts_when_missing() {
        let content = r#"<settings>
  <localRepository>/path</localRepository>
</settings>"#;
        let new_block = r#"<mirrors>
    <mirror>
      <id>new</id>
      <url>https://new.example.com</url>
    </mirror>
  </mirrors>"#;
        let result = replace_mirrors_block(content, new_block);
        assert!(result.contains("new.example.com"));
        assert!(result.contains("</settings>"));
    }

    #[test]
    fn build_settings_xml_contains_mirror_url() {
        let xml = build_settings_xml("aliyun", "https://maven.aliyun.com/repository/public/");
        assert!(xml.contains("aliyun"));
        assert!(xml.contains("https://maven.aliyun.com/repository/public/"));
        assert!(xml.contains("<mirrors>"));
    }
}
