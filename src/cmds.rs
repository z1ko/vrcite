use crate::types::BotError;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};
use teloxide::{
    dispatching::UpdateHandler, types::InlineKeyboardButton, types::InlineKeyboardMarkup,
};

// ChatId of the admin group
const ADMIN_GROUP_ID: ChatId = ChatId(-646467056);
// ChatId of the public channel
const PUBLIC_CHANNEL_ID: ChatId = ChatId(-1644783032);

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceiveCiteAuthor,
    ReceiveCiteText {
        author: String,
    },
    WaitUserApproval {
        author: String,
        citation: String,
    },
}

// Dialogue FSM
pub type BotDialogue = Dialogue<State, InMemStorage<State>>;
// Result of all handler functions
pub type BotResult = std::result::Result<(), BotError>;

// All available commands
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandi supportati")]
pub enum Command {
    #[command(description = "Mostra tutti i comandi disponibili")]
    Help,
    #[command(description = "Invia una nuova citazione")]
    Citazione,
    #[command(description = "Interrompi la creazione di una nuova citazione")]
    Cancella,
}

// Handles the Help command
async fn handler_help(bot: Bot, msg: Message) -> BotResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

const CITAZIONE_AUTHOR_STR: &str = "\
Di chi è questa citazione?
Invia il nome del professore nella forma Nome Cognome (es. Andrea Rossi):
";

// Handles the Citazione command
async fn handler_citazione(bot: Bot, dialogue: BotDialogue, msg: Message) -> BotResult {
    bot.send_message(msg.chat.id, CITAZIONE_AUTHOR_STR).await?;
    dialogue.update(State::ReceiveCiteAuthor).await?;
    Ok(())
}

// Handles the Cancella command
async fn handler_cancella(bot: Bot, dialogue: BotDialogue, msg: Message) -> BotResult {
    bot.send_message(msg.chat.id, "Cancello l'invio della citazione")
        .await?;
    dialogue.exit().await?;
    Ok(())
}

const ERROR_STR: &str = "\
Non riesco a processare il tuo messaggio.
Scrivi /help per vedere i comandi disponibili.
";

// Handles errors and incorrect state
async fn handler_error(bot: Bot, msg: Message) -> BotResult {
    bot.send_message(msg.chat.id, ERROR_STR).await?;
    Ok(())
}

const CITAZIONE_TESTO_STR: &str = "\
Cosa ha detto?
";

// Handles the author receive
async fn handler_citazione_autore(bot: Bot, dialogue: BotDialogue, msg: Message) -> BotResult {
    bot.send_message(msg.chat.id, CITAZIONE_TESTO_STR).await?;
    let author = msg.text().unwrap().to_string();
    dialogue.update(State::ReceiveCiteText { author }).await?;
    Ok(())
}

// Handles the text receive
async fn handler_citazione_text(
    bot: Bot,
    dialogue: BotDialogue,
    author: String,
    msg: Message,
) -> BotResult {
    let citation = msg.text().unwrap().to_string();

    let response = ["Ok", "Ricrea"].map(|p| InlineKeyboardButton::callback(p, p));
    bot.send_message(
        msg.chat.id,
        format!("{citation}\n\n- {author}, 2022\nLa citazione va bene così?"),
    )
    .reply_markup(InlineKeyboardMarkup::new([response]))
    .await?;

    dialogue
        .update(State::WaitUserApproval { author, citation })
        .await?;

    Ok(())
}

// Handle user approval
async fn handler_user_approval(
    bot: Bot,
    dialogue: BotDialogue,
    (author, citation): (String, String),
    query: CallbackQuery,
) -> BotResult {
    println!("User approval message");
    if let Some(choice) = &query.data {
        let text = format!("{citation}\n\n- {author}, 2022");

        if choice == "Ok" {
            const BUTTONS: [&str; 2] = ["Accetta", "Rifiuta"];
            let buttons = BUTTONS.map(|p| InlineKeyboardButton::callback(p, p));
            bot.send_message(ADMIN_GROUP_ID, text)
                .reply_markup(InlineKeyboardMarkup::new([buttons]))
                .await?;

            bot.send_message(
                dialogue.chat_id(),
                "Citazione inviata agli admin, se verrà approvata potrai vederla nel canale.",
            )
            .await?;
        }
        // Restart citation creation process
        else if choice == "Ricrea" {
            dialogue.update(State::ReceiveCiteAuthor).await?;
            bot.send_message(dialogue.chat_id(), "Ok, ricreiamo la citazione. Autore?")
                .await?;

            bot.answer_callback_query(query.id).await?;
            return Ok(());
        }
    }

    bot.answer_callback_query(query.id).await?;
    dialogue.exit().await?;
    Ok(())
}

// Handle admin approvral
async fn handler_admin_approval(bot: Bot, query: CallbackQuery) -> BotResult {
    println!("admin approval request");
    if let Some(choice) = &query.data {
        if let Some(message) = query.message {
            if choice == "Accetta" {
                println!("Citazione accettata!");
                bot.send_message(PUBLIC_CHANNEL_ID, message.text().unwrap().to_string())
                    .await?;
            }
            // Reject the citation
            else if choice == "Rifiuta" {
                println!("Citazione rifiutata!");
            }

            // Remove the message
            bot.delete_message(message.chat.id, message.id).await?;
        }
    }

    bot.answer_callback_query(query.id).await?;
    Ok(())
}

// Handles inline keyboards

// Generate commands handlers descriptor
pub fn schema() -> UpdateHandler<BotError> {
    use dptree::case;

    /* Descriptor for the commands */
    let commands = teloxide::filter_command::<Command, _>()
        .branch(
            /* All commands usable only in the start state */
            case![State::Start]
                .branch(case![Command::Help].endpoint(handler_help))
                .branch(case![Command::Citazione].endpoint(handler_citazione)),
        )
        /* All other commands */
        .branch(case![Command::Cancella].endpoint(handler_cancella));

    /* Generic message handler */
    let messager = Update::filter_message()
        .branch(commands)
        .branch(case![State::ReceiveCiteAuthor].endpoint(handler_citazione_autore))
        .branch(case![State::ReceiveCiteText { author }].endpoint(handler_citazione_text))
        .branch(dptree::endpoint(handler_error));

    /* All queries that have buttons */
    let queries = Update::filter_callback_query()
        .branch(case![State::WaitUserApproval { author, citation }].endpoint(handler_user_approval))
        .endpoint(handler_admin_approval);

    teloxide::dispatching::dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(messager)
        .branch(queries)
}
