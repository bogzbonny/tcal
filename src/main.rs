//use {
//    ollama_rs::{
//        coordinator::Coordinator,
//        generation::{
//            chat::ChatMessage,
//            parameters::{FormatType, JsonSchema, JsonStructure},
//        },
//        models::ModelOptions,
//        Ollama,
//    },
//    serde::Deserialize,
//    time::{macros::format_description, UtcDateTime},
//};

////#[derive(JsonSchema, Deserialize, Debug)]
//#[derive(Debug)]
//pub struct CalendarEntryInner {
//    pub datetime: UtcDateTime,
//    pub entry: String,
//}

//#[derive(JsonSchema, Deserialize, Debug)]
//pub struct CalendarEntry {
//    pub date: String,
//    pub time: String,
//    pub entry: String,
//}

///// global test memory
////static mut TEST_MEMORY: Vec<CalendarEntry> = Vec::new();

///// Add a new entry to the calendar.
/////
///// * date: The date of the entry in the format "YYYY-MM-DD".
///// * time: The 24 hour time of the entry in the format "HH:MM".
///// * entry: The entry to add to the calendar.
//#[ollama_rs::function]
//async fn add_to_calendar(
//    date: String,
//    time: String,
//    entry: String,
//) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
//    let dt = format!("{date} {time}");
//    let format = format_description!("[year]-[month]-[day] [hour]:[minute]");
//    let dt = UtcDateTime::parse(&dt, &format).unwrap();
//    let entry = CalendarEntryInner {
//        datetime: dt,
//        entry,
//    };
//    println!("Adding {entry:?}");

//    Ok(format!("Added {entry:?} to the calendar"))
//}

//#[tokio::main]
//async fn main() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
//    // compile all the arguments into a single string
//    let mut input: Vec<String> = std::env::args().collect();
//    // remove the first argument (the path to the binary)
//    input.remove(0);
//    let input = input.join(" ");
//    println!("Input: {input}");

//    let ollama = Ollama::default();

//    let history = vec![];

//    //let format = FormatType::StructuredJson(Box::new(JsonStructure::new::<CalendarEntry>()));

//    //let mut coordinator = Coordinator::new(ollama,"deepseek-r1:8b".to_string(), history)
//    //let mut coordinator = Coordinator::new(ollama, "smollm2:1.7b".to_string(), history)
//    //let mut coordinator = Coordinator::new(ollama, "llama3.2".to_string(), history)
//    let mut coordinator = Coordinator::new(ollama, "qwen3:8b".to_string(), history)
//        //.format(format)
//        .options(ModelOptions::default().temperature(0.0))
//        .add_tool(add_to_calendar);

//    let date_format = format_description!("[year]-[month]-[day]");
//    let today = UtcDateTime::now().format(date_format).unwrap().to_string();
//    let input =
//        format!("Today's date is {today} the user has just entered the following text: {input}");

//    let user_messages = vec![input];

//    for user_message in user_messages {
//        println!("User: {user_message}");

//        let user_message = ChatMessage::user(user_message.to_owned());
//        let resp = coordinator.chat(vec![user_message]).await?;
//        println!("Assistant: {}", resp.message.content);
//    }

//    Ok(())
//}

// ------------------------------------------

// -------------

use std::collections::HashMap;
use std::io::{stdout, Write};

use async_openai::config::OpenAIConfig;
use async_openai::types::{
    ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestMessage, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionToolArgs, ChatCompletionToolType,
    FunctionObjectArgs,
};
use async_openai::{types::CreateChatCompletionRequestArgs, Client};
use futures::StreamExt;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let client = Client::new();

    let user_prompt = "What's the weather like in Boston and Atlanta?";
    // This is the default host:port for Ollama's OpenAI endpoint.
    // Should match the config in docker-compose.yml.
    let api_base = "http://localhost:11434/v1";
    // Required but ignored
    let api_key = "ollama";

    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base),
    );

    println!("1");

    // This should match whatever model is downloaded in Ollama docker container.
    //let model = "llama3.2:latest";
    //let model = "qwen3:8b";
    let model = "smollm2:1.7b";

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u32)
        .model(model)
        .messages([ChatCompletionRequestUserMessageArgs::default()
            .content(user_prompt)
            .build()?
            .into()])
        .tools(vec![ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name("get_current_weather")
                    .description("Get the current weather in a given location")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "location": {
                                "type": "string",
                                "description": "The city and state, e.g. San Francisco, CA",
                            },
                            "unit": { "type": "string", "enum": ["celsius", "fahrenheit"] },
                        },
                        "required": ["location"],
                    }))
                    .build()?,
            )
            .build()?])
        .build()?;

    println!("2");

    let response_message = client
        .chat()
        .create(request)
        .await?
        .choices
        .first()
        .unwrap()
        .message
        .clone();

    println!("Response: {response_message:?}");

    if let Some(tool_calls) = response_message.tool_calls {
        let mut handles = Vec::new();
        for tool_call in tool_calls {
            let name = tool_call.function.name.clone();
            let args = tool_call.function.arguments.clone();
            let tool_call_clone = tool_call.clone();

            let handle =
                tokio::spawn(async move { call_fn(&name, &args).await.unwrap_or_default() });
            handles.push((handle, tool_call_clone));
        }

        let mut function_responses = Vec::new();

        for (handle, tool_call_clone) in handles {
            if let Ok(response_content) = handle.await {
                function_responses.push((tool_call_clone, response_content));
            }
        }

        let mut messages: Vec<ChatCompletionRequestMessage> =
            vec![ChatCompletionRequestUserMessageArgs::default()
                .content(user_prompt)
                .build()?
                .into()];

        let tool_calls: Vec<ChatCompletionMessageToolCall> = function_responses
            .iter()
            .map(|(tool_call, _response_content)| tool_call.clone())
            .collect();

        let assistant_messages: ChatCompletionRequestMessage =
            ChatCompletionRequestAssistantMessageArgs::default()
                .tool_calls(tool_calls)
                .build()?
                .into();

        let tool_messages: Vec<ChatCompletionRequestMessage> = function_responses
            .iter()
            .map(|(tool_call, response_content)| {
                ChatCompletionRequestToolMessageArgs::default()
                    .content(response_content.to_string())
                    .tool_call_id(tool_call.id.clone())
                    .build()
                    .unwrap()
                    .into()
            })
            .collect();

        messages.push(assistant_messages);
        messages.extend(tool_messages);

        let subsequent_request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u32)
            .model(model)
            .messages(messages)
            .build()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        let mut stream = client.chat().create_stream(subsequent_request).await?;

        let mut response_content = String::new();
        let mut lock = stdout().lock();
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    for chat_choice in response.choices.iter() {
                        if let Some(ref content) = chat_choice.delta.content {
                            write!(lock, "{}", content).unwrap();
                            response_content.push_str(content);
                        }
                    }
                }
                Err(err) => {
                    return Err(Box::new(err) as Box<dyn std::error::Error>);
                }
            }
        }
    }

    Ok(())
}

async fn call_fn(name: &str, args: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut available_functions: HashMap<&str, fn(&str, &str) -> serde_json::Value> =
        HashMap::new();
    available_functions.insert("get_current_weather", get_current_weather);

    println!("CALLED!!!!!!!!");

    let function_args: serde_json::Value = args.parse().unwrap();

    let location = function_args["location"].as_str().unwrap();
    let unit = function_args["unit"].as_str().unwrap_or("fahrenheit");
    let function = available_functions.get(name).unwrap();
    let function_response = function(location, unit);
    println!("RESPONSE: {function_response:?}");
    Ok(function_response)
}

fn get_current_weather(location: &str, unit: &str) -> serde_json::Value {
    let mut rng = thread_rng();

    let temperature: i32 = rng.gen_range(20..=55);

    let forecasts = [
        "sunny", "cloudy", "overcast", "rainy", "windy", "foggy", "snowy",
    ];

    let forecast = forecasts.choose(&mut rng).unwrap_or(&"sunny");

    let weather_info = json!({
        "location": location,
        "temperature": temperature.to_string(),
        "unit": unit,
        "forecast": forecast
    });

    weather_info
}
