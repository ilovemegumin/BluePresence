use crate::config::{AppConfig, PRO_NAMES};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use std::{
    sync::{
        Arc, RwLock,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{ProcessesToUpdate, System};

#[derive(Clone)]
struct VersionedConfig {
    revision: u64,
    value: AppConfig,
}

pub struct MonitorHandle {
    config: Arc<RwLock<VersionedConfig>>,
    shutdown_tx: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl MonitorHandle {
    pub fn start(config: AppConfig) -> Self {
        let config = Arc::new(RwLock::new(VersionedConfig {
            revision: 0,
            value: config,
        }));
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let worker_config = Arc::clone(&config);
        let thread = thread::spawn(move || monitor_loop(worker_config, shutdown_rx));

        Self {
            config,
            shutdown_tx: Some(shutdown_tx),
            thread: Some(thread),
        }
    }

    pub fn update_config(&self, value: AppConfig) {
        let mut config = self
            .config
            .write()
            .unwrap_or_else(|error| error.into_inner());
        config.value = value;
        config.revision = config.revision.wrapping_add(1);
    }

    fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for MonitorHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn monitor_loop(config: Arc<RwLock<VersionedConfig>>, shutdown_rx: mpsc::Receiver<()>) {
    let mut system = System::new();
    let mut discord: Option<DiscordIpcClient> = None;
    let mut started_at_ms = None;
    let mut published_revision = None;
    let mut last_publish: Option<Instant> = None;

    loop {
        system.refresh_processes(ProcessesToUpdate::All, true);
        let snapshot = config
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let game_running = system
            .processes()
            .values()
            .any(|process| process_matches(&process.name().to_string_lossy(), PRO_NAMES));

        if game_running {
            let start = *started_at_ms.get_or_insert_with(time);
            let needs_publish = published_revision != Some(snapshot.revision)
                || last_publish.is_none_or(|last| last.elapsed() >= Duration::from_secs(15));

            if discord.is_none() {
                let mut candidate = DiscordIpcClient::new("1519354091879530506");
                if candidate.connect().is_ok() {
                    discord = Some(candidate);
                }
            }

            if let Some(client) = discord.as_mut()
                && needs_publish
            {
                let payload = presence_activity(&snapshot.value, start);
                if client.set_activity(payload).is_ok() {
                    published_revision = Some(snapshot.revision);
                    last_publish = Some(Instant::now());
                } else {
                    discord = None;
                    published_revision = None;
                    last_publish = None;
                }
            }
        } else {
            if let Some(client) = discord.as_mut() {
                let _ = client.clear_activity();
                let _ = client.close();
            }
            discord = None;
            started_at_ms = None;
            published_revision = None;
            last_publish = None;
        }

        if shutdown_rx.recv_timeout(Duration::from_secs(2)).is_ok() {
            if let Some(client) = discord.as_mut() {
                let _ = client.clear_activity();
                let _ = client.close();
            }
            break;
        }
    }
}

fn presence_activity(config: &AppConfig, started_at_ms: i64) -> activity::Activity<'static> {
    let mut payload = activity::Activity::new()
        .activity_type(activity::ActivityType::Playing)
        .details(presence_details(config))
        .timestamps(activity::Timestamps::new().start(started_at_ms))
        .buttons(vec![activity::Button::new(
            "公式サイトからダウンロード",
            "https://bluearchive.jp/",
        )]);
    if let Some(state) = presence_state(config) {
        payload = payload.state(state);
    }
    payload
}

fn time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn process_matches(process_name: &str, configured_names: &[&str]) -> bool {
    configured_names
        .iter()
        .any(|name| process_name.eq_ignore_ascii_case(name.trim()))
}

fn presence_state(config: &AppConfig) -> Option<String> {
    (!config.friend_code.is_empty()).then(|| {
        format!("フレンドコード: {}", config.friend_code)
            .chars()
            .take(120)
            .collect()
    })
}

fn presence_details(config: &AppConfig) -> String {
    if config.player_name.is_empty() {
        "ブルーアーカイブをプレイ中".to_owned()
    } else {
        format!("名前: {}", config.player_name)
            .chars()
            .take(120)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro() {
        let names = ["BlueArchive.exe"];
        assert!(process_matches("bluearchive.EXE", &names));
        assert!(!process_matches("BlueArchiveLauncher.exe", &names));
    }

    #[test]
    fn profilelines() {
        assert_eq!(
            presence_details(&AppConfig::default()),
            "ブルーアーカイブをプレイ中"
        );
        assert_eq!(presence_state(&AppConfig::default()), None);

        let without_code = presence_state(&AppConfig {
            player_name: "アロナ".into(),
            ..Default::default()
        });
        let with_code_config = AppConfig {
            player_name: "アロナ".into(),
            friend_code: "ABC123".into(),
        };
        let code_only_config = AppConfig {
            friend_code: "ABC123".into(),
            ..Default::default()
        };

        assert_eq!(without_code, None);
        assert_eq!(presence_details(&with_code_config), "名前: アロナ");
        assert_eq!(
            presence_state(&with_code_config).as_deref(),
            Some("フレンドコード: ABC123")
        );
        assert_eq!(
            presence_details(&code_only_config),
            "ブルーアーカイブをプレイ中"
        );
        assert_eq!(
            presence_state(&code_only_config).as_deref(),
            Some("フレンドコード: ABC123")
        );
    }

    #[test]
    fn official_button() {
        let payload = serde_json::to_value(presence_activity(&AppConfig::default(), 123))
            .expect("activity must serialize");

        assert_eq!(payload["buttons"][0]["label"], "公式サイトからダウンロード");
        assert_eq!(payload["buttons"][0]["url"], "https://bluearchive.jp/");
    }
}
