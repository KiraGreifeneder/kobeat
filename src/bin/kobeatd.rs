use anyhow::Result;

fn main() -> Result<()> {
    kobeat::daemon::cli::run()
}
