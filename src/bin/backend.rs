use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    sync::RwLock,
    time::Duration,
};
use tokio::time;
use futures::stream::{self, StreamExt};
use chrono::{Utc, FixedOffset};

const FRONTENDS_FILE: &str = "frontends.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FrontendInfo {
    name: String,
    ip: String,
}

// For deletion, we only need the name.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DeleteFrontend {
    name: String,
}

// These types come from the frontend agent.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DiskUsage {
    mount_point: String,
    total: u64,
    used: u64,
    used_percent: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CpuInfo {
    name: String,
    cpu_usage: f32,
    frequency: u64,
}

// Updated SystemMetrics now includes memory information.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SystemMetrics {
    disk_usage: Vec<DiskUsage>,
    cpu_usage: f32,
    cpus: Vec<CpuInfo>,
    total_memory: u64,
    used_memory: u64,
    memory_percent: f64,
}

// Computed types defined only once.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ComputedDiskUsage {
    mount_point: String,
    total: u64,
    used: u64,
    used_percent: f64,
    status: String, // "red" if used_percent > 90, else "green"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ComputedCpuInfo {
    name: String,
    cpu_usage: f32,
    frequency: u64,
    status: String, // "red" if cpu_usage > 90, else "green"
}

// New computed type for memory usage.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ComputedMemoryUsage {
    total_memory: u64,
    used_memory: u64,
    memory_percent: f64,
    status: String, // "red" if memory_percent > 70, else "green"
}

// ServerUsage now includes memory usage, status fields, and crawl_time.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServerUsage {
    frontend: FrontendInfo,
    disk_usage: Option<Vec<ComputedDiskUsage>>,
    cpu_usage: Option<f32>,
    cpus: Option<Vec<ComputedCpuInfo>>,
    memory_usage: Option<ComputedMemoryUsage>,
    disk_status: String,    // "red" if any disk is red, else "green"
    cpu_status: String,     // "red" if global CPU usage > 70, else "green"
    memory_status: String,  // "red" if memory usage > 70, else "green"
    overall_status: String, // "red" if any of the statuses is red, else "green"
    crawl_time: String,     // crawl time in Thailand time (UTC+7)
}

// Global in‑memory storage.
static FRONTENDS: Lazy<RwLock<Vec<FrontendInfo>>> = Lazy::new(|| {
    let frontends = load_frontends().unwrap_or_else(|_| vec![]);
    RwLock::new(frontends)
});
static USAGE_DATA: Lazy<RwLock<Vec<ServerUsage>>> = Lazy::new(|| RwLock::new(vec![]));

fn load_frontends() -> std::io::Result<Vec<FrontendInfo>> {
    let mut file = File::open(FRONTENDS_FILE)?;
    let mut data = String::new();
    file.read_to_string(&mut data)?;
    let frontends = serde_json::from_str(&data)?;
    Ok(frontends)
}

fn save_frontends(frontends: &Vec<FrontendInfo>) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(frontends)?;
    let mut file = File::create(FRONTENDS_FILE)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}

#[get("/api/servers")]
async fn api_servers() -> impl Responder {
    let usage_data = USAGE_DATA.read().unwrap().clone();
    HttpResponse::Ok().json(usage_data)
}

#[get("/")]
async fn index() -> impl Responder {
    // The HTML uses Bootstrap 5.
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>Monitoring Dashboard</title>
  <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
  <style>
    body { padding: 20px; }
    .server-container { border: 1px solid #dee2e6; border-radius: 0.25rem; padding: 15px; margin-bottom: 15px; }
    .server-header { display: flex; justify-content: space-between; align-items: center; }
    .status-label { margin-left: 10px; font-weight: bold; }
    .green { color: green; }
    .red { color: red; }
    .tab-group { margin-top: 10px; }
    .tab-item { margin-bottom: 10px; }
    .tab { cursor: pointer; padding: 5px 10px; border: 1px solid #dee2e6; border-radius: 0.25rem; background-color: #f8f9fa; margin-right: 5px; }
    .tab:hover { background-color: #e9ecef; }
    .tab-content { margin-top: 5px; display: none; }
  </style>
</head>
<body>
  <div class="container">
    <h1 class="mb-4">Monitoring Dashboard</h1>
    <div id="alert-container"></div>
    <button id="addFrontendBtn" class="btn btn-primary mb-3">Add New Frontend</button>
    <div id="servers"></div>
  </div>

  <!-- Add Frontend Modal -->
  <div class="modal fade" id="addFrontendModal" tabindex="-1" aria-labelledby="addFrontendModalLabel" aria-hidden="true">
    <div class="modal-dialog">
      <div class="modal-content">
        <form id="add-frontend-form">
          <div class="modal-header">
            <h5 class="modal-title" id="addFrontendModalLabel">Add New Frontend</h5>
            <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
          </div>
          <div class="modal-body">
            <div class="mb-3">
              <label for="frontendName" class="form-label">Server Name</label>
              <input type="text" class="form-control" id="frontendName" name="name" required>
            </div>
            <div class="mb-3">
              <label for="frontendIP" class="form-label">Frontend IP</label>
              <input type="text" class="form-control" id="frontendIP" name="ip" required>
            </div>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">Cancel</button>
            <button type="submit" class="btn btn-primary">Add Frontend</button>
          </div>
        </form>
      </div>
    </div>
  </div>

  <script src="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/js/bootstrap.bundle.min.js"></script>
  <script>
    // Global object to hold each server's expanded state.
    window.expandedStates = {};

    // Helper function to compute relative time from a crawl_time string.
    function computeTimeDisplay(crawlTimeString) {
      let crawlTimeISO = crawlTimeString.replace(" ", "T");
      let crawlTime = new Date(crawlTimeISO);
      let now = new Date();
      let diffMs = now - crawlTime;
      let diffSeconds = Math.floor(diffMs / 1000);
      if (diffSeconds == 0) {
        return "(Just now)";
      } else {
        return `(${diffSeconds} seconds ago)`;
      }
    }

    // Function to update all relative time displays.
    function updateAllRelativeTimes() {
      let timeDisplays = document.getElementsByClassName('time-display');
      for (let td of timeDisplays) {
        let crawlTime = td.getAttribute('data-crawl-time');
        td.textContent = computeTimeDisplay(crawlTime);
      }
    }

    // Update the relative times every second.
    setInterval(updateAllRelativeTimes, 1000);

    function showAlert(message, type = 'success') {
      const alertContainer = document.getElementById('alert-container');
      const alertDiv = document.createElement('div');
      alertDiv.className = `alert alert-${type} alert-dismissible fade show`;
      alertDiv.role = 'alert';
      alertDiv.innerHTML = `
        ${message}
        <button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>
      `;
      alertContainer.appendChild(alertDiv);
      setTimeout(() => {
        const bsAlert = new bootstrap.Alert(alertDiv);
        bsAlert.close();
      }, 3000);
    }

    function renderServers(serversData) {
      const container = document.getElementById('servers');
      container.innerHTML = '';
      serversData.forEach(srv => {
        const { frontend, disk_usage } = srv;
        const connectivity = srv.disk_usage ? 'green' : 'red';
        const overallStatus = srv.overall_status;
        const diskStatus = srv.disk_status;
        const cpuStatus = srv.cpu_status;
        const memoryStatus = srv.memory_status;
        
        const serverDiv = document.createElement('div');
        serverDiv.className = 'server-container';
        
        // Header.
        const headerDiv = document.createElement('div');
        headerDiv.className = 'server-header';
        const infoSpan = document.createElement('span');
        infoSpan.className = 'server-info';
        // Static part: server name and IP.
        infoSpan.textContent = `${frontend.name} (IP: ${frontend.ip})`;
        // Create a span to hold the relative time.
        let timeSpan = document.createElement('span');
        timeSpan.className = 'time-display';
        timeSpan.setAttribute('data-crawl-time', srv.crawl_time);
        timeSpan.style.marginLeft = "10px";
        timeSpan.textContent = computeTimeDisplay(srv.crawl_time);
        infoSpan.appendChild(timeSpan);
        infoSpan.style.cursor = 'pointer';
        headerDiv.appendChild(infoSpan);
        
        const deleteBtn = document.createElement('button');
        deleteBtn.className = 'btn btn-sm btn-danger';
        deleteBtn.textContent = 'Delete';
        deleteBtn.addEventListener('click', () => {
          if (confirm("Are you sure you want to delete this frontend?")) {
            deleteFrontend(frontend.name);
          }
        });
        headerDiv.appendChild(deleteBtn);
        
        const statusContainer = document.createElement('span');
        const connectivitySpan = document.createElement('span');
        connectivitySpan.className = `status-label ${connectivity}`;
        connectivitySpan.textContent = `[Connectivity: ${connectivity === 'green' ? 'OK' : 'Down'}]`;
        statusContainer.appendChild(connectivitySpan);
        const overallSpan = document.createElement('span');
        overallSpan.className = `status-label ${overallStatus}`;
        const overallIcon = overallStatus === 'green'
          ? '<span class="green">&#x2714;</span>'
          : '<span class="red">&#x26A0;</span>';
        overallSpan.innerHTML = `[Overall: ${overallIcon}]`;
        statusContainer.appendChild(overallSpan);
        headerDiv.appendChild(statusContainer);
        serverDiv.appendChild(headerDiv);
        
        // Tab group container.
        const tabGroup = document.createElement('div');
        tabGroup.className = 'tab-group';
        tabGroup.style.display = (window.expandedStates[frontend.name] && window.expandedStates[frontend.name] !== "") ? 'block' : 'none';

        // Toggle tabGroup display when clicking on the header.
        infoSpan.addEventListener('click', () => {
          if (tabGroup.style.display === 'none') {
            tabGroup.style.display = 'block';
            if (!window.expandedStates[frontend.name] || window.expandedStates[frontend.name] === "") {
              window.expandedStates[frontend.name] = 'open';
            }
          } else {
            tabGroup.style.display = 'none';
            window.expandedStates[frontend.name] = '';
          }
        });
        
        // Disk Usage tab-item.
        const diskTabItem = document.createElement('div');
        diskTabItem.className = 'tab-item';
        const diskTab = document.createElement('div');
        diskTab.className = 'tab';
        const diskTabIcon = diskStatus === 'red'
          ? '<span class="red">&#x26A0;</span>'
          : '<span class="green">&#x2714;</span>';
        diskTab.innerHTML = `Disk Usage ${diskTabIcon}`;
        diskTab.addEventListener('click', () => {
          if (window.expandedStates[frontend.name] === 'disk') {
            window.expandedStates[frontend.name] = 'open';
            diskContent.style.display = 'none';
          } else {
            window.expandedStates[frontend.name] = 'disk';
            diskContent.style.display = 'block';
            cpuContent.style.display = 'none';
            memoryContent.style.display = 'none';
          }
        });
        diskTabItem.appendChild(diskTab);
        const diskContent = document.createElement('div');
        diskContent.id = `disk-content-${frontend.name}`;
        diskContent.className = 'tab-content';
        if (disk_usage) {
          let tableHtml = `<table class="table table-striped">
            <thead>
              <tr>
                <th>Mount Point</th>
                <th>Total (bytes)</th>
                <th>Used (bytes)</th>
                <th>Usage %</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>`;
          disk_usage.forEach(disk => {
            tableHtml += `<tr>
              <td>${disk.mount_point}</td>
              <td>${disk.total}</td>
              <td>${disk.used}</td>
              <td>${disk.used_percent.toFixed(2)}%</td>
              <td><span class="text-${disk.status}">${disk.status === 'red' ? '&#x26A0;' : '&#x2714;'}</span></td>
            </tr>`;
          });
          tableHtml += `</tbody></table>`;
          diskContent.innerHTML = tableHtml;
        } else {
          diskContent.innerHTML = `<p class="text-danger">Unable to retrieve disk usage data.</p>`;
        }
        diskContent.style.display = (window.expandedStates[frontend.name] === 'disk') ? 'block' : 'none';
        diskTabItem.appendChild(diskContent);
        tabGroup.appendChild(diskTabItem);
        
        // CPU Usage tab-item.
        const cpuTabItem = document.createElement('div');
        cpuTabItem.className = 'tab-item';
        const cpuTab = document.createElement('div');
        cpuTab.className = 'tab';
        const cpuTabIcon = cpuStatus === 'red'
          ? '<span class="red">&#x26A0;</span>'
          : '<span class="green">&#x2714;</span>';
        cpuTab.innerHTML = `CPU Usage ${cpuTabIcon}`;
        cpuTab.addEventListener('click', () => {
          if (window.expandedStates[frontend.name] === 'cpu') {
            window.expandedStates[frontend.name] = 'open';
            cpuContent.style.display = 'none';
          } else {
            window.expandedStates[frontend.name] = 'cpu';
            cpuContent.style.display = 'block';
            diskContent.style.display = 'none';
            memoryContent.style.display = 'none';
          }
        });
        cpuTabItem.appendChild(cpuTab);
        const cpuContent = document.createElement('div');
        cpuContent.id = `cpu-content-${frontend.name}`;
        cpuContent.className = 'tab-content';
        let cpuHtml = "";
        if (srv.cpu_usage !== undefined && srv.cpu_usage !== null) {
          cpuHtml += `<p>Global CPU Usage: ${srv.cpu_usage.toFixed(2)}%</p>`;
        }
        if (srv.cpus && srv.cpus.length > 0) {
          cpuHtml += `<table class="table table-striped">
            <thead>
              <tr>
                <th>CPU Core</th>
                <th>Usage (%)</th>
                <th>Frequency (MHz)</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>`;
          srv.cpus.forEach(cpu => {
            cpuHtml += `<tr>
              <td>${cpu.name}</td>
              <td>${cpu.cpu_usage.toFixed(2)}</td>
              <td>${cpu.frequency}</td>
              <td><span class="text-${cpu.status}">${cpu.status === 'red' ? '&#x26A0;' : '&#x2714;'}</span></td>
            </tr>`;
          });
          cpuHtml += `</tbody></table>`;
        } else {
          cpuHtml += `<p class="text-danger">Unable to retrieve CPU usage data.</p>`;
        }
        cpuContent.innerHTML = cpuHtml;
        cpuContent.style.display = (window.expandedStates[frontend.name] === 'cpu') ? 'block' : 'none';
        cpuTabItem.appendChild(cpuContent);
        tabGroup.appendChild(cpuTabItem);
        
        // Memory Usage tab-item.
        const memoryTabItem = document.createElement('div');
        memoryTabItem.className = 'tab-item';
        const memoryTab = document.createElement('div');
        memoryTab.className = 'tab';
        const memoryTabIcon = memoryStatus === 'red'
          ? '<span class="red">&#x26A0;</span>'
          : '<span class="green">&#x2714;</span>';
        memoryTab.innerHTML = `Memory Usage ${memoryTabIcon}`;
        memoryTab.addEventListener('click', () => {
          if (window.expandedStates[frontend.name] === 'memory') {
            window.expandedStates[frontend.name] = 'open';
            memoryContent.style.display = 'none';
          } else {
            window.expandedStates[frontend.name] = 'memory';
            memoryContent.style.display = 'block';
            diskContent.style.display = 'none';
            cpuContent.style.display = 'none';
          }
        });
        memoryTabItem.appendChild(memoryTab);
        const memoryContent = document.createElement('div');
        memoryContent.id = `memory-content-${frontend.name}`;
        memoryContent.className = 'tab-content';
        let memoryHtml = "";
        if (srv.memory_usage) {
          memoryHtml += `<p>Total Memory: ${srv.memory_usage.total_memory}</p>`;
          memoryHtml += `<p>Used Memory: ${srv.memory_usage.used_memory}</p>`;
          memoryHtml += `<p>Usage: ${srv.memory_usage.memory_percent.toFixed(2)}%</p>`;
        } else {
          memoryHtml += `<p class="text-danger">Unable to retrieve memory usage data.</p>`;
        }
        memoryContent.innerHTML = memoryHtml;
        memoryContent.style.display = (window.expandedStates[frontend.name] === 'memory') ? 'block' : 'none';
        memoryTabItem.appendChild(memoryContent);
        tabGroup.appendChild(memoryTabItem);
        
        serverDiv.appendChild(tabGroup);
        container.appendChild(serverDiv);
      });
    }
    
    async function refreshData() {
      try {
        const res = await fetch('/api/servers');
        const data = await res.json();
        renderServers(data);
      } catch (err) {
        console.error('Error fetching server data:', err);
      }
    }
    
    async function addFrontend(event) {
      event.preventDefault();
      const formData = new FormData(document.getElementById('add-frontend-form'));
      try {
        const res = await fetch('/add_frontend', {
          method: 'POST',
          headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
          body: new URLSearchParams({
            name: formData.get('name'),
            ip: formData.get('ip')
          })
        });
        if (res.ok) {
          // Clear the form inputs after successful addition.
          document.getElementById('add-frontend-form').reset();
          const modalEl = document.getElementById('addFrontendModal');
          const modal = bootstrap.Modal.getInstance(modalEl);
          modal.hide();
          showAlert('Frontend added successfully!', 'success');
          refreshData();
        } else {
          showAlert('Error adding frontend: ' + await res.text(), 'danger');
        }
      } catch (err) {
        showAlert('Error adding frontend: ' + err, 'danger');
      }
    }
    
    async function deleteFrontend(name) {
      try {
        const res = await fetch('/delete_frontend', {
          method: 'POST',
          headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
          body: new URLSearchParams({ name })
        });
        if (res.ok) {
          showAlert('Frontend deleted successfully!', 'success');
          refreshData();
        } else {
          showAlert('Error deleting frontend: ' + await res.text(), 'danger');
        }
      } catch (err) {
        showAlert('Error deleting frontend: ' + err, 'danger');
      }
    }
    
    document.getElementById('addFrontendBtn').addEventListener('click', () => {
      new bootstrap.Modal(document.getElementById('addFrontendModal')).show();
    });
    document.getElementById('add-frontend-form').addEventListener('submit', addFrontend);
    
    refreshData();
    setInterval(refreshData, 5000);
  </script>
</body>
</html>
"#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

#[post("/add_frontend")]
async fn add_frontend(form: web::Form<FrontendInfo>) -> impl Responder {
    let info = form.into_inner();
    let mut frontends = FRONTENDS.write().unwrap();
    if frontends.iter().any(|f| f.name == info.name) {
        return HttpResponse::BadRequest().body("Frontend name already exists");
    }
    frontends.push(info.clone());
    if let Err(e) = save_frontends(&frontends) {
        eprintln!("Failed to save frontends: {}", e);
    }
    HttpResponse::Ok().body("Added")
}

#[post("/delete_frontend")]
async fn delete_frontend(form: web::Form<DeleteFrontend>) -> impl Responder {
    let info = form.into_inner();
    let mut frontends = FRONTENDS.write().unwrap();
    frontends.retain(|f| f.name != info.name);
    if let Err(e) = save_frontends(&frontends) {
        eprintln!("Failed to save frontends: {}", e);
    }
    HttpResponse::Ok().body("Deleted")
}

async fn poll_frontends() {
    let client = Client::new();
    loop {
        let frontends = FRONTENDS.read().unwrap().clone();

        let new_usage_data: Vec<ServerUsage> = stream::iter(frontends.into_iter())
            .map(|fe| {
                let client = client.clone();
                async move {
                    // Get current time in Thailand (UTC+7)
                    let crawl_time = Utc::now()
                        .with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap())
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();

                    let url = format!("http://{}:8081/usage", fe.ip);
                    let usage = match client.get(&url).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            match resp.json::<SystemMetrics>().await {
                                Ok(metrics) => {
                                    let computed_disks: Vec<ComputedDiskUsage> =
                                        metrics.disk_usage.into_iter().map(|d| {
                                            ComputedDiskUsage {
                                                mount_point: d.mount_point,
                                                total: d.total,
                                                used: d.used,
                                                used_percent: d.used_percent,
                                                status: if d.used_percent > 90.0 {
                                                    "red".to_string()
                                                } else {
                                                    "green".to_string()
                                                },
                                            }
                                        }).collect();
                                    let computed_cpus: Vec<ComputedCpuInfo> =
                                        metrics.cpus.into_iter().map(|c| {
                                            ComputedCpuInfo {
                                                name: c.name,
                                                cpu_usage: c.cpu_usage,
                                                frequency: c.frequency,
                                                status: if c.cpu_usage > 90.0 {
                                                    "red".to_string()
                                                } else {
                                                    "green".to_string()
                                                },
                                            }
                                        }).collect();
                                    let computed_memory = ComputedMemoryUsage {
                                        total_memory: metrics.total_memory,
                                        used_memory: metrics.used_memory,
                                        memory_percent: metrics.memory_percent,
                                        status: if metrics.memory_percent > 90.0 {
                                            "red".to_string()
                                        } else {
                                            "green".to_string()
                                        },
                                    };
                                    let disk_status = if computed_disks.iter().any(|d| d.status == "red") {
                                        "red"
                                    } else {
                                        "green"
                                    }.to_string();
                                    let cpu_status = if metrics.cpu_usage > 90.0 {
                                        "red"
                                    } else {
                                        "green"
                                    }.to_string();
                                    let memory_status = computed_memory.status.clone();
                                    let overall_status = if disk_status == "red" || cpu_status == "red" || memory_status == "red" {
                                        "red"
                                    } else {
                                        "green"
                                    }.to_string();
                                    ServerUsage {
                                        frontend: fe.clone(),
                                        disk_usage: Some(computed_disks),
                                        cpu_usage: Some(metrics.cpu_usage),
                                        cpus: Some(computed_cpus),
                                        memory_usage: Some(computed_memory),
                                        disk_status,
                                        cpu_status,
                                        memory_status,
                                        overall_status,
                                        crawl_time: crawl_time.clone(),
                                    }
                                },
                                Err(err) => {
                                    eprintln!("Failed to parse JSON for {}: {}", fe.name, err);
                                    ServerUsage {
                                        frontend: fe.clone(),
                                        disk_usage: None,
                                        cpu_usage: None,
                                        cpus: None,
                                        memory_usage: None,
                                        disk_status: "red".to_string(),
                                        cpu_status: "red".to_string(),
                                        memory_status: "red".to_string(),
                                        overall_status: "red".to_string(),
                                        crawl_time: crawl_time.clone(),
                                    }
                                }
                            }
                        },
                        Err(err) => {
                            eprintln!("Error contacting frontend {}: {}", fe.name, err);
                            ServerUsage {
                                frontend: fe.clone(),
                                disk_usage: None,
                                cpu_usage: None,
                                cpus: None,
                                memory_usage: None,
                                disk_status: "red".to_string(),
                                cpu_status: "red".to_string(),
                                memory_status: "red".to_string(),
                                overall_status: "red".to_string(),
                                crawl_time: crawl_time.clone(),
                            }
                        },
                        _ => ServerUsage {
                            frontend: fe.clone(),
                            disk_usage: None,
                            cpu_usage: None,
                            cpus: None,
                            memory_usage: None,
                            disk_status: "red".to_string(),
                            cpu_status: "red".to_string(),
                            memory_status: "red".to_string(),
                            overall_status: "red".to_string(),
                            crawl_time: crawl_time.clone(),
                        }
                    };
                    usage
                }
            })
            .buffered(100)
            .collect()
            .await;

        {
            let mut usage_data = USAGE_DATA.write().unwrap();
            *usage_data = new_usage_data;
        }
        time::sleep(Duration::from_secs(5)).await;
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tokio::spawn(async {
        poll_frontends().await;
    });
    println!("Backend server running on http://0.0.0.0:8080");
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(api_servers)
            .service(add_frontend)
            .service(delete_frontend)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
