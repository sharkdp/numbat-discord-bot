use numbat::markup::Formatter;
use numbat::markup::PlainTextFormatter;
use numbat::module_importer::BuiltinModuleImporter;
use numbat::pretty_print::PrettyPrint;
use numbat::resolver::CodeSource;
use numbat::Context as NumbatContext;

use poise::serenity_prelude as serenity;
use std::sync::Arc;
use std::sync::Mutex;

struct BotState {
    numbat: Arc<Mutex<NumbatContext>>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

type Context<'a> = poise::Context<'a, BotState, Error>;

/// Runs Numbat code
#[poise::command(slash_command, prefix_command)]
async fn numbat(
    ctx: Context<'_>,
    #[description = "Numbat code"] input: String,
) -> Result<(), Error> {
    let output = {
        let mut numbat = ctx.data().numbat.lock().unwrap();

        let result = numbat.interpret(&input, CodeSource::Text);

        let formatter = PlainTextFormatter {};

        match result {
            Ok((statements, result)) => {
                let statement = statements.last();

                let pretty = if let Some(statement) = statement {
                    formatter.format(&statement.pretty_print(), true)
                } else {
                    "".into()
                };

                let markup = result.to_markup(statement, numbat.dimension_registry(), true, true);
                let output = formatter.format(&markup, false);

                format!(">>> {input}\n\n{pretty}\n\n{output}")
            }
            Err(e) => format!("Error: {}", e),
        }
    };

    ctx.say(format!("```\n{output}```")).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let mut numbat_ctx = NumbatContext::new(BuiltinModuleImporter {});
    let _ = numbat_ctx.interpret("use prelude", CodeSource::Internal);
    numbat_ctx.load_currency_module_on_demand(true);

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![numbat()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(BotState {
                    numbat: Arc::new(Mutex::new(numbat_ctx)),
                })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
