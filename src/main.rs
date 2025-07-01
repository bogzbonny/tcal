use {
    async_openai::{
        config::OpenAIConfig,
        types::{
            ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
            ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
            ChatCompletionToolArgs, ChatCompletionToolType, CreateChatCompletionRequestArgs,
            FunctionObjectArgs,
        },
        Client,
    },
    serde_json::{json, Value},
    std::collections::HashMap,
    time::{macros::format_description, Duration, Time, UtcDateTime, Weekday},
};

//#[derive(JsonSchema, Deserialize, Debug)]
#[derive(Debug)]
pub struct CalendarEntry {
    pub datetime: UtcDateTime,
    pub entry: String,
}

pub fn basic_improve_user_prompt(mut user_prompt: String) -> String {
    let day_week_prefix = vec![
        "mon", "tue", "wed", "thu", "fri", "sat", "sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat",
        "Sun",
    ];
    for prefix in day_week_prefix {
        user_prompt =
            user_prompt.replace(&format!("next {prefix}"), &format!("next week on {prefix}"));
        user_prompt =
            user_prompt.replace(&format!("this {prefix}"), &format!("this week on {prefix}"));
    }
    user_prompt
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let client = Client::new();

    // compile all the arguments into a single string
    let mut input: Vec<String> = std::env::args().collect();
    // remove the first argument (the path to the binary)
    input.remove(0);
    let user_prompt = input.join(" ");
    let user_prompt = basic_improve_user_prompt(user_prompt);
    println!("user_prompt: {}", &user_prompt);

    let now = UtcDateTime::now();
    let weekday_format = format_description!("[weekday]");
    let date_format = format_description!("[year]-[month]-[day]");
    let date = now.format(date_format).unwrap().to_string();
    let weekday = now.format(weekday_format).unwrap().to_string();

    let api_base = "http://localhost:11434/v1";
    let api_key = "ollama";

    let client = Client::with_config(
        OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base),
    );

    // This should match whatever model is downloaded in Ollama docker container.

    //let model = "llama3.2:1b";
    //let model = "qwen3:8b";
    //let model = "smollm2:1.7b";
    //let model = "phi4-mini:latest";
    let model = "llama3.2:latest"; // <--- the best model so far

    let system_prompt =
        format!("You are a calendar assistant. Today's date is {date}, which is a {weekday}.",);

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u32)
        .model(model)
        .temperature(1.0)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default().content(system_prompt).build()?.into(),
            ChatCompletionRequestUserMessageArgs::default()
            .content(user_prompt.clone())
            .build()?
            .into()])
        .tools(vec![ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(
                FunctionObjectArgs::default()
                    .name("add_to_calendar_absolute")
                    .description("Add a new entry to the calendar using absolute date and time")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "date": {
                                "type": "string",
                                "description": "The date of the entry in the format YYYY-MM-DD",
                            },
                            "time": {
                                "type": "string",
                                "description": "The 24 hour time of the entry in the format HH:MM",
                            },
                            "entry": {
                                "type": "string",
                                "description": "The entry to add to the calendar",
                            },
                        },
                        "required": ["date", "time", "entry"],
                    }))
                    .build()?,
            )
            .function(
                FunctionObjectArgs::default()
                    .name("add_to_calendar_relative")
                    .description("Add a new entry to the calendar using relative date, all days and weeks will be added together")
                    .parameters(json!({
                        "type": "object",
                        "properties": {
                            "in_days": {
                                "type": "integer",
                                "description": "The number of days in the future when the event occurs",
                            },
                            "in_weeks": {
                                "type": "integer",
                                "description": "The number of weeks in the future when the event occurs",
                            },
                            "on_day": {
                                "type": "string",
                                "description": "The day of the week when the event occurs, or \"tomorrow\" to add the event to the next day",
                            },
                            "time": {
                                "type": "string",
                                "description": "The 24 hour time of the entry in the format HH:MM",
                            },
                            "entry": {
                                "type": "string",
                                "description": "The entry to add to the calendar",
                            },
                        },
                        "required": ["time", "entry"],
                    }))
                    .build()?,
            )
            .build()?])
        .build()?;

    let mut tool_event_date_list: Vec<String> = vec![];

    for _ in 0..=5 {
        let response_message = client
            .chat()
            .create(request.clone())
            .await?
            .choices
            .first()
            .unwrap()
            .message
            .clone();

        println!("Response: {response_message:?}\n");

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
                    .content(user_prompt.clone())
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
                    tool_event_date_list.push(response_content.to_string());
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
        }
    }
    println!("events: {tool_event_date_list:?}");

    //let mut verifications: Vec<String> = vec![];

    //for tool_event_date in tool_event_date_list {
    //    let verify_user_prompt = format!(
    //    "The user has just made a prompt to create an event: \"{user_prompt}\", Given that \
    //    today's date is {date}, which is a {weekday}. Does this event take place on {tool_event_date}? \
    //    Respond ONLY with TRUE or FALSE");

    //    //println!("verify_user_prompt: {}", &verify_user_prompt);
    //    let request = CreateChatCompletionRequestArgs::default()
    //        .max_tokens(512u32)
    //        .model(model)
    //        .temperature(1.0)
    //        .messages([ChatCompletionRequestUserMessageArgs::default()
    //            .content(verify_user_prompt.clone())
    //            .build()?
    //            .into()])
    //        .build()?;

    //    let response_message = client
    //        .chat()
    //        .create(request)
    //        .await?
    //        .choices
    //        .first()
    //        .unwrap()
    //        .message
    //        .clone();

    //    //println!(
    //    //    "verify_user_prompt Response: {}",
    //    //    response_message.content.unwrap()
    //    //);
    //    verifications.push(response_message.content.unwrap());
    //}
    //println!("verifications: {verifications:?}");

    Ok(())
}

async fn call_fn(name: &str, args: &str) -> Result<Value, Box<dyn std::error::Error>> {
    #[allow(clippy::type_complexity)]
    let mut available_functions: HashMap<&str, fn(&str, &str, &str) -> serde_json::Value> =
        HashMap::new();
    available_functions.insert("add_to_calendar_absolute", add_to_calendar_abs);

    //println!("CALLED!!!!!!!!");

    match name {
        "add_to_calendar_absolute" => {
            let function_args: serde_json::Value = args.parse().unwrap();
            let date_ = function_args["date"].as_str().unwrap();
            let time_ = function_args["time"].as_str().unwrap_or("00:00");
            let entry = function_args["entry"].as_str().unwrap();
            let val = add_to_calendar_abs(date_, time_, entry);
            println!("RESPONSE: {val:?}");
            Ok(val)
        }
        "add_to_calendar_relative" => {
            let function_args: serde_json::Value = args.parse().unwrap();
            let in_days = match function_args["in_days"].as_i64() {
                Some(days) => Some(days),
                None => {
                    // attempt to parse as a string
                    let days = function_args["in_days"]
                        .as_str()
                        .map(|s| s.parse().unwrap_or(0));
                    days
                }
            };
            let in_weeks = match function_args["in_weeks"].as_i64() {
                Some(weeks) => Some(weeks),
                None => {
                    // attempt to parse as a string
                    let weeks = function_args["in_weeks"]
                        .as_str()
                        .map(|s| s.parse().unwrap_or(0));
                    weeks
                }
            };
            let on_day = function_args["on_day"].as_str().map(|s| s.to_string());
            let time_ = function_args["time"].as_str().unwrap_or("00:00");
            let entry = function_args["entry"].as_str().unwrap_or("");
            let val = add_to_calendar_rel(in_days, in_weeks, on_day, time_, entry);
            //println!("RESPONSE: {val:?}");
            Ok(val)
        }
        _ => {
            println!("Unknown function: {}", name);
            Ok(json!({}))
        }
    }
}

fn add_to_calendar_abs(date: &str, _time: &str, _entry: &str) -> serde_json::Value {
    //let dt = format!("{date} {time}");
    //let format = format_description!("[year]-[month]-[day] [hour]:[minute]");
    //let dt = UtcDateTime::parse(&dt, &format).unwrap();
    //let cal_entry = CalendarEntry {
    //    datetime: dt,
    //    entry: entry.to_string(),
    //};
    //println!("Adding {cal_entry:?}");

    let entry_info = json!(date);
    entry_info
}

// add to calendar relative
fn add_to_calendar_rel(
    in_days: Option<i64>,
    in_weeks: Option<i64>,
    on_day: Option<String>,
    time: &str,
    _entry: &str,
) -> serde_json::Value {
    let mut dt = UtcDateTime::now();

    match (in_days, in_weeks) {
        (Some(days), Some(weeks)) => {
            // for silly llm mistakes where is provides both in_days and in_weeks
            if weeks * 7 == days {
                dt = dt.checked_add(Duration::days(days)).unwrap();
            }
        }
        (Some(days), None) => {
            dt = dt.checked_add(Duration::days(days)).unwrap();
        }
        (None, Some(weeks)) => {
            dt = dt.checked_add(Duration::weeks(weeks)).unwrap();
        }
        (None, None) => {}
    }

    // correct the day if necessary
    if let Some(mut on_day) = on_day {
        use std::str::FromStr;
        //cleanup the string
        on_day.retain(|c| !c.is_whitespace());
        on_day.make_ascii_lowercase();
        // set the first letter to uppercase
        if let Some(first_letter) = on_day.chars().next() {
            on_day.replace_range(0..1, &first_letter.to_string().to_uppercase());
        }

        // remove the final letter if it is 's'
        if on_day.ends_with("s") {
            on_day.pop();
        }

        let on_day = match on_day.as_str() {
            "Mon" => "Monday",
            "Tue" => "Tuesday",
            "Tues" => "Tuesday",
            "Wed" => "Wednesday",
            "Wednessday" => "Wednesday",
            "Thu" => "Thursday",
            "Thur" => "Thursday",
            "Fri" => "Friday",
            "Sat" => "Saturday",
            "Satur" => "Saturday",
            "Sun" => "Sunday",
            _ => on_day.as_str(),
        };
        if let Ok(on_day) = Weekday::from_str(on_day) {
            let existing_day = dt.weekday();
            if existing_day != on_day {
                let days_to_add = on_day.number_days_from_sunday() as i8
                    - existing_day.number_days_from_sunday() as i8;
                dt = dt.checked_add(Duration::days(days_to_add as i64)).unwrap();
            }
        }
        if on_day == "tomorrow" || on_day == "Tomorrow" {
            dt = UtcDateTime::now();
            dt = dt.checked_add(Duration::days(1)).unwrap();
        }
        //println!("ON_DAY: {on_day}");
    }

    // parse the time from the user input
    let format = format_description!("[hour]:[minute]");
    let dt = if let Ok(time_) = Time::parse(time, &format) {
        // replace the time with the parsed time
        dt.replace_time(time_)
    } else {
        dt
    };

    //let cal_entry = CalendarEntry {
    //    datetime: dt,
    //    entry: entry.to_string(),
    //};
    //println!("Adding {cal_entry:?}");

    let date_format = format_description!("[year]-[month]-[day]");

    let entry_info = json!(dt.format(date_format).unwrap());
    entry_info
}

//fn get_current_weather(location: &str, unit: &str) -> serde_json::Value {
//    let mut rng = thread_rng();

//    let temperature: i32 = rng.gen_range(20..=55);

//    let forecasts = [
//        "sunny", "cloudy", "overcast", "rainy", "windy", "foggy", "snowy",
//    ];

//    let forecast = forecasts.choose(&mut rng).unwrap_or(&"sunny");

//    let weather_info = json!({
//        "location": location,
//        "temperature": temperature.to_string(),
//        "unit": unit,
//        "forecast": forecast
//    });

//    weather_info
//}
