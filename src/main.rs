use anyhow::{Result};
use colored::Colorize;
use clap::{Parser, Subcommand};

mod scraper;
mod types;
mod util;
mod sources;

#[derive(Parser, Debug)]
#[command(name = "av", version, about = "AV CLI: 搜索、查看与下载番号和演员作品", long_about = None)]
struct Cli {
    /// 统一输出为 JSON
    #[arg(long, global = true)]
    json: bool,

    /// 输出调试日志
    #[arg(long, global = true)]
    debug: bool,

    /// 只显示无马赛克（基于标题/标签的启发式判断）
    #[arg(long = "uncen", short = 'u', alias = "nomo", global = true)]
    uncen: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 获取该番号对应的磁力链接
    #[command(visible_alias = "get")]
    Install { code: String },

    /// 展示该番号的详细信息
    Detail { code: String },

    /// 列出该演员的所有番号
    #[command(visible_alias = "ls")]
    List { actor: String },

    /// 搜索演员或番号
    Search { query: String },

    /// 查看最新的番（默认 20 条）
    Top { #[arg(short, long, default_value_t = 20)] limit: usize },

    /// 演员热度排行榜（分页）
    Actors { #[arg(short, long, default_value_t = 1)] page: usize, #[arg(short='n', long, default_value_t = 50)] per_page: usize },

    /// 在浏览器中打开观看视频
    #[command(visible_alias = "see")]
    View { code: String },

    /// 自动更新到最新版本
    #[command(name = "update", visible_alias = "self-update")]
    SelfUpdate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    util::set_debug(cli.debug);

    match cli.command {
        Commands::Install { code } => {
            let detail = scraper::fetch_detail(&code).await?;
            
            // 显示所有可用的磁力链接，按种子数排序
            println!("番号: {} - {}", code.bold(), detail.title);
            
            if detail.magnet_infos.is_empty() && detail.magnets.is_empty() {
                println!("{}", "未找到可用的磁力链接".red().bold());
            } else {
                println!("\n{}", "可用磁力链接:".green().bold());
                
                // 先显示有详细信息的磁力链接
                if !detail.magnet_infos.is_empty() {
                    // 按种子数排序
                    let mut sorted_magnets = detail.magnet_infos.clone();
                    sorted_magnets.sort_by(|a, b| b.seeders.unwrap_or(0).cmp(&a.seeders.unwrap_or(0)));
                    
                    for (i, m) in sorted_magnets.iter().enumerate() {
                        let mut info = String::new();
                        if let Some(name) = &m.name { info.push_str(&format!(" | {}", name)); }
                        if let Some(size) = &m.size { info.push_str(&format!(" | {}", size)); }
                        if let Some(res) = &m.resolution { info.push_str(&format!(" | {}", res)); }
                        if let Some(codec) = &m.codec { info.push_str(&format!(" | {}", codec)); }
                        if let Some(b) = m.avg_bitrate_mbps { info.push_str(&format!(" | ~{:.2} Mbps", b)); }
                        if let Some(s) = m.seeders { info.push_str(&format!(" | S:{}", s)); }
                        if let Some(lc) = m.leechers { info.push_str(&format!(" | L:{}", lc)); }
                        
                        println!("{}. {}{}", i+1, m.url.cyan(), info);
                    }
                } else {
                    // 显示简单的磁力链接
                    for (i, magnet) in detail.magnets.iter().enumerate() {
                        println!("{}. {}", i+1, magnet.cyan());
                    }
                }
                
                println!("\n{}", "使用方法:".yellow().bold());
                println!("- 复制链接到您的 BT 客户端");
                println!("- 或使用命令行工具: aria2c \"<磁力链接>\"");
            }
            
            Ok(())
        }
        Commands::Detail { code } => {
            util::debug(format!("detail: fetching {}", code));
            let detail = scraper::fetch_detail(&code).await?;
            if cli.json {
                util::print_output(&detail, true);
            } else {
                util::print_detail_human(&detail);
            }
            Ok(())
        }
        Commands::List { actor } => {
            let mut items = scraper::list_actor_titles(&actor).await?;
            if cli.uncen {
                items.retain(|i| util::looks_uncensored(&i.title));
            }
            if cli.json {
                util::print_output(&items, true);
            } else {
                util::print_items_table(&items);
            }
            Ok(())
        }
        Commands::Search { query } => {
            let mut items = scraper::search(&query).await?;
            if cli.uncen {
                items.retain(|i| util::looks_uncensored(&i.title));
            }
            if cli.json {
                util::print_output(&items, true);
            } else {
                util::print_items_table(&items);
            }
            Ok(())
        }
        Commands::Top { limit } => {
            let mut items = scraper::top(limit).await?;
            if cli.uncen {
                items.retain(|i| util::looks_uncensored(&i.title));
            }
            if cli.json {
                util::print_output(&items, true);
            } else {
                util::print_items_table(&items);
            }
            Ok(())
        }
        Commands::Actors { page, per_page } => {
            let (actors, total) = scraper::actors(page, per_page, cli.uncen).await?;
            if cli.json {
                util::print_output(&(actors, total), true);
            } else {
                util::print_actors_table(&actors, page, per_page, total);
            }
            Ok(())
        }
        Commands::View { code } => {
            util::debug(format!("view: finding play URL for {}", code));
            let play_url = scraper::get_play_url(&code).await?;
            println!("Opening browser to watch: {}", play_url);
            util::open_browser_url(&play_url).await?;
            Ok(())
        }
        Commands::SelfUpdate => {
            util::self_update().await?;
            Ok(())
        }
    }
}
