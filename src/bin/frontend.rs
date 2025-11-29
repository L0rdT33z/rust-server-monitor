use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};

#[derive(Serialize)]
struct DiskUsage {
    mount_point: String,
    total: u64,
    used: u64,
    used_percent: f64,
}

#[derive(Serialize)]
struct CpuInfo {
    name: String,
    cpu_usage: f32,
    frequency: u64,
}

#[derive(Serialize)]
struct SystemMetrics {
    disk_usage: Vec<DiskUsage>,
    cpu_usage: f32,
    cpus: Vec<CpuInfo>,
    total_memory: u64,
    used_memory: u64,
    memory_percent: f64,
}

#[get("/usage")]
async fn get_disk_usage() -> impl Responder {
    let mut sys = System::new_all();
    sys.refresh_all();

    let disk_info: Vec<DiskUsage> = sys.disks()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let used_percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            DiskUsage {
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total,
                used,
                used_percent,
            }
        })
        .collect();

    let cpu_usage = sys.global_cpu_info().cpu_usage();
    let cpus: Vec<CpuInfo> = sys.cpus()
        .iter()
        .map(|cpu| CpuInfo {
            name: cpu.name().to_string(),
            cpu_usage: cpu.cpu_usage(),
            frequency: cpu.frequency(),
        })
        .collect();

    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let memory_percent = if total_memory > 0 {
        (used_memory as f64 / total_memory as f64) * 100.0
    } else {
        0.0
    };

    let metrics = SystemMetrics {
        disk_usage: disk_info,
        cpu_usage,
        cpus,
        total_memory,
        used_memory,
        memory_percent,
    };
    HttpResponse::Ok().json(metrics)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Frontend agent running on http://0.0.0.0:8081");
    HttpServer::new(|| {
        App::new().service(get_disk_usage)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
