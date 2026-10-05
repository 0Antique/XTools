use super::{pinyin::normalize, Application, SearchResult};

const TOOLS: &[(&str, &str, &[&str], &str)] = &[
    (
        "clipboard",
        "剪贴板管理",
        &["剪贴板", "clipboard", "cb", "jiantieban", "jtb"],
        "文本、图片和文件历史",
    ),
    (
        "color",
        "屏幕取色",
        &["取色", "颜色", "color", "picker", "quse", "qs"],
        "复制屏幕像素的颜色",
    ),
    (
        "rename",
        "批量重命名",
        &["重命名", "rename", "batch", "rn", "chongmingming", "cmm"],
        "预览、安全重命名与撤销",
    ),
    (
        "settings",
        "设置",
        &["设置", "setting", "settings", "shezhi", "sz"],
        "快捷键和本地偏好",
    ),
];

fn matches(name: &str, full: &str, initials: &str, query: &str) -> u16 {
    if name == query {
        1000
    } else if name.starts_with(query) {
        900
    } else if !initials.is_empty() && initials == query {
        880
    } else if !initials.is_empty() && initials.starts_with(query) {
        850
    } else if !full.is_empty() && full.starts_with(query) {
        820
    } else if name.contains(query) {
        700
    } else if !full.is_empty() && full.contains(query) {
        650
    } else if query.chars().count() >= 2 && (subsequence(query, name) || subsequence(query, full)) {
        500
    } else {
        0
    }
}

fn subsequence(query: &str, candidate: &str) -> bool {
    let mut wanted = query.chars();
    let mut next = wanted.next();
    for ch in candidate.chars() {
        if next == Some(ch) {
            next = wanted.next();
            if next.is_none() {
                return true;
            }
        }
    }
    next.is_none()
}

fn app_result(app: &Application) -> SearchResult {
    SearchResult {
        id: app.id.clone(),
        name: app.name.clone(),
        kind: "application".into(),
        icon_path: app.icon_path.clone(),
        subtitle: Some(app.launch_target.clone()),
    }
}

fn tool_result(tool: &(&str, &str, &[&str], &str)) -> SearchResult {
    SearchResult {
        id: tool.0.into(),
        name: tool.1.into(),
        kind: "tool".into(),
        icon_path: None,
        subtitle: Some(tool.3.into()),
    }
}

pub fn search(apps: &[Application], query: &str, show_recent: bool) -> Vec<SearchResult> {
    let query = normalize(query.trim());
    if query.is_empty() {
        let mut recent: Vec<_> = apps
            .iter()
            .filter(|app| app.launch_count > 0 && app.last_launched_at.is_some())
            .collect();
        recent.sort_by(|a, b| {
            b.last_launched_at
                .cmp(&a.last_launched_at)
                .then_with(|| a.normalized_name.cmp(&b.normalized_name))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut results = if show_recent {
            recent.into_iter().take(8).map(app_result).collect()
        } else {
            Vec::new()
        };
        results.extend(TOOLS.iter().map(tool_result));
        return results;
    }
    // Match quality is the primary key. Frequency/recency only resolve equally
    // accurate matches, so even an extreme usage count cannot beat an exact hit.
    let mut ranked: Vec<(u16, u8, i64, SearchResult)> = apps
        .iter()
        .filter_map(|app| {
            let score = matches(
                &app.normalized_name,
                &app.pinyin,
                &app.pinyin_initials,
                &query,
            );
            (score > 0).then(|| {
                (
                    score,
                    (64 - app.launch_count.saturating_add(1).leading_zeros()).min(20) as u8,
                    app.last_launched_at.unwrap_or(0),
                    app_result(app),
                )
            })
        })
        .collect();
    for tool in TOOLS {
        let score = tool
            .2
            .iter()
            .map(|alias| matches(alias, "", "", &query))
            .chain(std::iter::once(matches(tool.1, "", "", &query)))
            .max()
            .unwrap_or(0);
        if score > 0 {
            ranked.push((score, 0, 0, tool_result(tool)));
        }
    }
    ranked.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.3.name.cmp(&b.3.name))
            .then_with(|| a.3.id.cmp(&b.3.id))
    });
    ranked.into_iter().take(60).map(|entry| entry.3).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn application(name: &str) -> Application {
        Application::new(
            name.into(),
            "startMenu",
            "executable",
            name.into(),
            name.into(),
        )
    }
    #[test]
    fn chinese_full_and_initial_queries_find_wechat() {
        let apps = vec![application("微信")];
        for query in ["微信", "weixin", "wx", "wei", "weix", " WX "] {
            assert_eq!(search(&apps, query, true)[0].name, "微信");
        }
    }
    #[test]
    fn match_quality_always_precedes_usage() {
        let mut frequent = application("微信开发者工具");
        frequent.launch_count = u64::MAX;
        frequent.last_launched_at = Some(i64::MAX);
        assert_eq!(
            search(&[frequent, application("微信")], "微信", true)[0].name,
            "微信"
        );
    }
    #[test]
    fn recents_only_include_launched_apps_and_are_capped_at_eight() {
        let mut apps = vec![application("Never launched")];
        for i in 1..=12 {
            let mut app = application(&format!("App {i}"));
            app.launch_count = 1;
            app.last_launched_at = Some(i);
            apps.push(app);
        }
        let found = search(&apps, "", true);
        assert_eq!(found.iter().filter(|r| r.kind == "application").count(), 8);
        assert_eq!(found[0].name, "App 12");
        assert_eq!(search(&apps, "", false).len(), 4);
    }
    #[test]
    fn tool_aliases_and_deterministic_ties() {
        assert_eq!(search(&[], "cb", true)[0].id, "clipboard");
        assert_eq!(search(&[], "picker", true)[0].id, "color");
        assert_eq!(search(&[], "rn", true)[0].id, "rename");
        let a = application("Alpha 1");
        let b = application("Alpha 2");
        assert_eq!(
            search(&[b.clone(), a.clone()], "alpha", true)[0].id,
            search(&[a, b], "alpha", true)[0].id
        );
    }
}
