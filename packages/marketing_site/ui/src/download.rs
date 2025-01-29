use std::sync::LazyLock;

use gigachad_transformer_models::{AlignItems, LayoutDirection};
use maud::{html, Markup};
use regex::Regex;

use crate::page;

#[must_use]
pub fn download() -> Markup {
    page(&html! {
        div sx-align-items=(AlignItems::Center) {
            div sx-width="100%" sx-max-width=(1000) {
                h1 sx-border-bottom="2, #ccc" sx-padding-bottom=(20) sx-margin-bottom=(10) { "Downloads" }
                div sx-hidden=(true) hx-get="/releases" hx-trigger="load" {}
            }
        }
    })
}

#[derive(Default, Clone, Debug)]
pub struct Os<'a> {
    pub lower_name: &'a str,
    pub name: &'a str,
    pub header: &'a str,
}

#[derive(Default, Clone, Debug)]
pub struct OsRelease<'a> {
    pub version: &'a str,
    pub published_at: &'a str,
    pub url: &'a str,
    pub assets: Vec<OsAsset<'a>>,
}

#[derive(Default, Clone, Debug)]
pub struct OsAsset<'a> {
    pub name: &'a str,
    pub asset: Option<FileAsset<'a>>,
    pub other_formats: Vec<FileAsset<'a>>,
}

#[derive(Default, Clone, Debug)]
pub struct FileAsset<'a> {
    pub browser_download_url: &'a str,
    pub name: &'a str,
    pub size: u64,
}

fn format_class_name(value: &str) -> String {
    static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w-]").unwrap());
    REGEX
        .replace_all(&value.split_whitespace().collect::<Vec<_>>().join("-"), "_")
        .to_lowercase()
}

fn get_os_header(asset: &str) -> &str {
    match asset {
        "windows" => "Windows",
        "mac_intel" => "macOS",
        "linux" => "Linux",
        "android" => "Android",
        _ => asset,
    }
}

fn format_size(size: u64) -> String {
    bytesize::to_string(size, true)
}

#[must_use]
pub fn releases(releases: &[OsRelease], os: &Os) -> Markup {
    html! {
        div id="releases" {
            @for release in releases {
                div id=(format_class_name(release.version)) sx-padding-y=(20) {
                    h2 sx-dir=(LayoutDirection::Row) sx-align-items=(AlignItems::End) {
                        div { "Release " (release.version) }
                        div sx-font-size=(16) sx-margin-left=(10) sx-margin-bottom=(2) sx-color="#ccc" {
                            (release.published_at)
                        }
                        div sx-font-size=(16) sx-margin-left=(10) sx-margin-bottom=(2) {
                            "[" a sx-color="#fff" target="_blank" href=(release.url) { "GitHub" } "]"
                        }
                    }
                    @for release_asset in &release.assets {
                        @if let Some(asset) = &release_asset.asset {
                            div {
                                @if os.lower_name == release_asset.name {
                                    div sx-color="#888" {
                                        "// We think you are running " (os.header)
                                    }
                                }
                                h3 { (get_os_header(release_asset.name)) }
                                "Download "
                                a sx-color="#fff" href=(asset.browser_download_url) { (asset.name) }
                                " (" (format_size(asset.size)) ")"
                                ul sx-margin=(0) {
                                    @for other_asset in &release_asset.other_formats {
                                        li {
                                            a sx-color="#fff" href=(other_asset.browser_download_url) { (other_asset.name) }
                                            " (" (format_size(other_asset.size)) ")"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
