//! gcloud helpers: list GCP projects, list GKE clusters and fetch kubectl credentials.
//! Everything runs on a worker thread and reports back through `K8sEvent`.

use crate::auth::gcloud_command;
use crate::k8s::K8sEvent;
use serde_json::Value;
use std::sync::mpsc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GcpProject {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GkeCluster {
    pub name: String,
    pub location: String,
    pub status: String,
}

fn run_gcloud(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = gcloud_command(args)
        .output()
        .map_err(|e| format!("เรียก gcloud ไม่สำเร็จ: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "gcloud ทำงานไม่สำเร็จ".to_string()
        } else {
            err
        });
    }
    Ok(out.stdout)
}

fn run_gcloud_json(args: &[&str]) -> Result<Value, String> {
    let stdout = run_gcloud(args)?;
    serde_json::from_slice(&stdout).map_err(|e| format!("อ่านผลลัพธ์ gcloud ไม่ได้: {e}"))
}

fn str_field(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}

pub fn parse_projects(v: &Value) -> Vec<GcpProject> {
    let mut list: Vec<GcpProject> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| {
                    let id = str_field(p, "projectId");
                    if id.is_empty() {
                        return None;
                    }
                    let name = str_field(p, "name");
                    Some(GcpProject {
                        name: if name.is_empty() { id.clone() } else { name },
                        id,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list
}

pub fn parse_clusters(v: &Value) -> Vec<GkeCluster> {
    let mut list: Vec<GkeCluster> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let name = str_field(c, "name");
                    if name.is_empty() {
                        return None;
                    }
                    Some(GkeCluster {
                        name,
                        location: str_field(c, "location"),
                        status: str_field(c, "status"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list
}

pub fn spawn_list_projects(tx: mpsc::Sender<K8sEvent>, wake: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let result = run_gcloud_json(&["projects", "list", "--format=json"])
            .map(|v| parse_projects(&v));
        let _ = tx.send(K8sEvent::Projects(result));
        wake();
    });
}

pub fn spawn_list_clusters(
    project: String,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let result = run_gcloud_json(&[
            "container",
            "clusters",
            "list",
            "--project",
            project.as_str(),
            "--format=json",
        ])
        .map(|v| parse_clusters(&v));
        let _ = tx.send(K8sEvent::Clusters { project, result });
        wake();
    });
}

pub fn spawn_get_credentials(
    project: String,
    cluster: GkeCluster,
    tx: mpsc::Sender<K8sEvent>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let result = run_gcloud(&[
            "container",
            "clusters",
            "get-credentials",
            cluster.name.as_str(),
            "--location",
            cluster.location.as_str(),
            "--project",
            project.as_str(),
        ])
        .map(|_| ());
        let _ = tx.send(K8sEvent::Credentials {
            project,
            cluster: cluster.name,
            result,
        });
        wake();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_projects_sorted_by_name() {
        let v: Value = serde_json::from_str(
            r#"[{"projectId":"b-id","name":"Zeta"},{"projectId":"a-id","name":"Alpha"},{"name":"no id"}]"#,
        )
        .unwrap();
        let list = parse_projects(&v);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "a-id");
        assert_eq!(list[1].name, "Zeta");
    }

    #[test]
    fn parses_clusters() {
        let v: Value = serde_json::from_str(
            r#"[{"name":"prod","location":"asia-southeast1","status":"RUNNING"}]"#,
        )
        .unwrap();
        let list = parse_clusters(&v);
        assert_eq!(list[0].location, "asia-southeast1");
        assert_eq!(list[0].status, "RUNNING");
    }
}
