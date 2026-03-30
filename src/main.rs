use anyhow::Result;
use swiftgit::ui;

#[tokio::main]
async fn main() -> Result<()> {
    ui::run_tui().await
}
