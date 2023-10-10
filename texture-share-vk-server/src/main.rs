use std::{
    fs::{self, OpenOptions},
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, SystemTime},
};

use clap::Parser;
use fs2::FileExt;
use texture_share_vk_server::VkServer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "test.lock")]
    lock_file: String,

    #[arg(short, long, default_value = "test.sock")]
    socket_file: String,

    #[arg(long, default_value = "shared_image_")]
    shmem_prefix: String,

    #[arg(short, long, default_value_t = 2000)]
    timeout_millis: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let lock_file_path = Path::new(&args.lock_file);
    fs::create_dir_all(&lock_file_path.parent().unwrap_or(Path::new(".")))?;

    // Take ownership of lock_file
    let lock_file = {
        const LOCK_FILE_TIMEOUT: Duration = Duration::from_millis(2000);
        let stop_time = SystemTime::now() + LOCK_FILE_TIMEOUT;
        loop {
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(lock_file_path)?;
            let lock_res = file.try_lock_exclusive();

            if lock_res.is_ok() {
                break Ok(file);
            }

            if SystemTime::now() > stop_time {
                break Err(lock_res.err().unwrap());
            }
        }
    }
    .expect(&format!(
        "Failed to acquire lock for file {:?}",
        lock_file_path
    ));

    let _ = fs::remove_file(&args.socket_file);

    let vk_server = VkServer::new(
        &args.socket_file,
        &args.shmem_prefix,
        Duration::from_millis(args.timeout_millis),
    )?;

    vk_server.loop_server(Arc::new(AtomicBool::new(false)))?;

    // File cleanup
    let _ = fs::remove_file(&args.socket_file);
    let _ = fs::remove_file(lock_file_path);
    lock_file.unlock()?;

    Ok(())
}
