use std::sync::Arc;

use clap::Parser;
use log::*;
use teloxide::requests::Requester;
use tokio_cron_scheduler::{Job, JobScheduler};

mod api;
use api::{HNApi, HNItem};
mod bot;
use bot::HNBot;

#[derive(Parser, Debug)]
struct Flags {
    #[clap(long)]
    token: String,
    #[clap(long)]
    id: Option<i64>,
    #[clap(long, default_value = "0 */15 * * * *")]
    schedule: String,
}

async fn main_impl(args: Flags) -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let bot = teloxide::Bot::new(args.token);
    let id = if let Some(id) = args.id {
        id
    } else {
        info!("Send a message to the bot to get your chat id, and pass it with the --id flag");
        teloxide::repl(
            bot,
            |bot: teloxide::Bot, msg: teloxide::types::Message| async move {
                info!("Chat id is {}", msg.chat.id);
                bot.send_message(msg.chat.id, format!("Your chat id is {}", msg.chat.id))
                    .await?;
                Ok(())
            },
        )
        .await;
        return Ok(());
    };
    let hn_bot = HNBot::new(bot, id);
    let hn_bot = Arc::new(tokio::sync::Mutex::new(hn_bot));

    info!("Started");
    let sched = JobScheduler::new().await?;
    let job = Job::new_async(args.schedule.as_str(), move |_, _| {
        let hn_bot = hn_bot.clone();
        Box::pin(async move {
            let mut hn_bot = hn_bot.lock().await;
            if let Err(e) = hn_bot.refresh_and_send().await {
                error!("Error while executing refresh/send: {}", e);
            }
        })
    })?;
    sched.add(job).await?;

    sched.shutdown_on_ctrl_c();
    sched.start().await?;

    tokio::signal::ctrl_c().await?;
    info!("Shutting down");

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Flags::parse();
    if let Err(e) = main_impl(args).await {
        error!("{}", e);
        std::process::exit(1)
    }
}
