use {
    ollama_rs::{
        coordinator::Coordinator,
        generation::{
            chat::ChatMessage,
            parameters::{FormatType, JsonSchema, JsonStructure},
        },
        models::ModelOptions,
        Ollama,
    },
    serde::Deserialize,
};

#[allow(dead_code)]
#[derive(JsonSchema, Deserialize, Debug)]
struct CalendarEntry {
    entry: String,
    date: String,
}

/// Add a new entry to the calendar.
///
/// * city - City to get the weather for.
#[ollama_rs_macros::function]
async fn add_to_calendar_absolute(
    date: String,
    entry: String,
) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
    Ok(format!("Added {entry} to the calendar on {date}"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    // compile all the arguments into a single string
    let input: Vec<String> = std::env::args().collect();
    let input = input.join(" ");
    println!("Input: {input}");

    let ollama = Ollama::default();

    let test_calendar = vec![
        CalendarEntry {
            entry: "Test entry".to_string(),
            date: "2023-05-01".to_string(),
        },
        CalendarEntry {
            entry: "Another test entry".to_string(),
            date: "2023-05-02".to_string(),
        },
    ];

    let history = vec![];

    let format = FormatType::StructuredJson(Box::new(JsonStructure::new::<CalendarEntry>()));

    let mut coordinator = Coordinator::new(ollama, "llama3.2".to_string(), history)
        .format(format)
        .options(ModelOptions::default().temperature(0.0))
        .add_tool(add_to_calendar);

    let user_messages = vec![input];

    for user_message in user_messages {
        println!("User: {user_message}");

        let user_message = ChatMessage::user(user_message.to_owned());
        let resp = coordinator.chat(vec![user_message]).await?;
        println!("Assistant: {}", resp.message.content);
    }

    Ok(())
}
