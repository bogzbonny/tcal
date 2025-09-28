use {
    ollama_rs::{
        generation::{
            completion::request::GenerationRequest,
            parameters::{FormatType, JsonSchema, JsonStructure},
        },
        models::ModelOptions,
        Ollama,
    },
    serde::Deserialize,
    std::cmp::PartialEq,
    std::fmt::Debug,
    time::UtcDateTime,
    //serde_json::json,
    time::{Date, Duration, Month, OffsetDateTime, Time, UtcOffset, Weekday as TimeWeekday},
};

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Time24Hr {
    #[schemars(regex(pattern = r"^(?:[0-9]|1[0-9]|2[0-3])$"))]
    pub hour: u8,
    #[schemars(regex(pattern = r"^(?:[0-9]|[0-5][0-9])$"))]
    pub minute: u8,
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum AmPm {
    Am,
    Pm,
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TimeAmPm {
    #[schemars(regex(pattern = r"^(?:[0-9]|1[0-2])$"))]
    pub hour: u8,
    #[schemars(regex(pattern = r"^(?:0[0-9]|[0-5][0-9])$"))]
    pub minute: u8,
    pub am_pm: AmPm,
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum CalTime {
    Unspecified,
    TimeWith24Hr(Time24Hr),
    TimeWithAmPm(TimeAmPm),
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CalDate {
    #[schemars(regex(pattern = r"^\d{4}$"))]
    pub year: u16,
    #[schemars(regex(pattern = r"^(?:0[1-9]|1[0-2])$"))]
    pub month: u8,
    #[schemars(regex(pattern = r"^(?:0?[1-9]|[12][0-9]|3[01])$"))]
    pub day: u8,
}

impl CalDate {
    pub fn to_date(&self) -> Date {
        let month = Month::January.nth_next(self.month - 1);
        Date::from_calendar_date(self.year as i32, month, self.day as u8).unwrap()
    }
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MonthDay {
    #[schemars(regex(pattern = r"^(?:0[1-9]|1[0-2])$"))]
    pub month: u8,
    #[schemars(regex(pattern = r"^(?:0?[1-9]|[12][0-9]|3[01])$"))]
    pub day: u8,
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl From<&Weekday> for TimeWeekday {
    fn from(weekday: &Weekday) -> Self {
        match weekday {
            Weekday::Monday => TimeWeekday::Monday,
            Weekday::Tuesday => TimeWeekday::Tuesday,
            Weekday::Wednesday => TimeWeekday::Wednesday,
            Weekday::Thursday => TimeWeekday::Thursday,
            Weekday::Friday => TimeWeekday::Friday,
            Weekday::Saturday => TimeWeekday::Saturday,
            Weekday::Sunday => TimeWeekday::Sunday,
        }
    }
}

#[derive(JsonSchema, Deserialize, Debug, PartialEq, Eq, Clone)]
enum When {
    NextWeek(Weekday),
    ThisWeek(Weekday),
    InExactDays(i64),
    MonthDay(MonthDay),
    AbsoluteDate(CalDate),
}

impl When {
    /// Gets the date relative to now given `When` information
    /// Returns the date without the time component and the current local offset.
    pub fn get_date(&self) -> (Date, UtcOffset) {
        // Current local date and offset
        let now = OffsetDateTime::now_local().expect("Unable to get local time");
        let offset = now.offset();
        let today = now.date();

        let target_date = match self {
            When::NextWeek(weekday) => {
                // Days until next occurrence of the specified weekday
                let weekday: TimeWeekday = weekday.into();
                let weekday_num = weekday.number_from_monday() as i64;
                let now_weekday_num = now.weekday().number_from_monday() as i64;
                let mut diff = (weekday_num - now_weekday_num + 7) % 7;
                if diff == 0 {
                    diff = 7; // Ensure "next" means at least a week ahead
                }
                today + Duration::days(diff)
            }
            When::ThisWeek(weekday) => {
                // Days relative to this week's specified weekday
                let weekday: TimeWeekday = weekday.into();
                let weekday_num = weekday.number_from_monday() as i64;
                let now_weekday_num = now.weekday().number_from_monday() as i64;
                let diff = weekday_num - now_weekday_num;
                today + Duration::days(diff)
            }
            When::InExactDays(days) => {
                // Add or subtract the given number of days
                today + Duration::days(*days)
            }
            When::MonthDay(md) => {
                // Resolve month/day within the current year or next year
                let year = today.year();
                let month = Month::January.nth_next(md.month - 1);
                let mut date = Date::from_calendar_date(year, month, md.day)
                    .expect("Invalid month/day combination");
                if date < today {
                    date = Date::from_calendar_date(year + 1, month, md.day)
                        .expect("Invalid month/day combination");
                }
                date
            }
            When::AbsoluteDate(cal) => cal.to_date(),
        };

        (target_date, offset)
    }
}

impl Default for When {
    fn default() -> Self {
        Self::InExactDays(0)
    }
}

#[derive(JsonSchema, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
struct Namer {
    event_name: String,
}

//pub struct Location {
//    location: String,
//}
#[derive(JsonSchema, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
enum Location {
    #[default]
    None,
    Location(String),
}

impl Location {
    fn cleanup(&mut self) {
        match &self {
            Location::Location(s)
                if s == "Unknown"
                    || s == "unknown"
                    || s == "None"
                    || s == "none"
                    || s == "Not Specified"
                    || s == "not specified" =>
            {
                *self = Location::None;
            }
            _ => {}
        }
    }
}

#[derive(JsonSchema, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Names {
    names: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ollama = Ollama::default();
    //let model = "llama3.2:latest".to_string();
    //let model = "phi3.5".to_string();
    //let model = "deepseek-r1:8b".to_string();
    //let model = "gpt-oss:20b".to_string();
    //let model = "qwen3:1.7b".to_string(); // too small!
    //let model = "qwen3:4b".to_string();
    let model = "qwen3:8b".to_string();

    // compile all the arguments into a single string
    let mut input: Vec<String> = std::env::args().collect();
    // remove the first argument (the path to the binary)
    input.remove(0);
    let user_prompt = input.join(" ");
    let user_prompt = basic_improve_user_prompt(user_prompt);
    println!("user_prompt: {}", &user_prompt);

    //let now = UtcDateTime::now();
    //let weekday_format = format_description!("[weekday]");
    //let date_format = format_description!("[year]-[month]-[day]");
    //let date = now.format(date_format).unwrap().to_string();
    //let weekday = now.format(weekday_format).unwrap().to_string();
    let sys_prompt = "You are a calendar assistant".to_string();

    let chooser_prompt = format!(
        "The user has just made a prompt to create an event: \"{user_prompt}\", Given the user's \
prompt, call one of the following functions: 
- NextWeek
  - Call this if the user has said \"next week\", \"next monday\", etc.
- ThisWeek
  - Call this if the user has said \"this week\", \"this monday\", etc.
- InExactDays
  - Call this if the user has said \"in 2 days\", \"in 5 days\", \"tomorrow\", \"today\", etc. 
    DO NOT call this if the user has said \"next week\", \"this week\", etc.
- MonthDay
  - Call this if the user has explicitly provided a month and day for the event.
- AbsoluteDate
  - Add a new entry to the calendar using absolute date and time. DO NOT call this if \
    the user has provided a relative date information. ONLY EVER call this if \
    the user has provided a month and day and a year. 
Respond only in JSON.
        "
    );
    dbg!(&chooser_prompt);

    let _when = process::<When>(&ollama, &model, &sys_prompt, &chooser_prompt, true).await?;

    let chooser_prompt = format!(
        "The user has just made a prompt to create an event: \"{user_prompt}\", Given the user's \
        prompt, what is the name of the event? DO NOT include the date or time information in the \
        name. ONLY USE words used by the user prompt. Respond only in JSON."
    );
    dbg!(&chooser_prompt);
    let _namer = process::<Namer>(&ollama, &model, &sys_prompt, &chooser_prompt, true).await?;

    //let chooser_prompt = format!(
    //    "Given the event details: \"{user_prompt}\", where is the location of the event? If the \
    //    event says \"go to X\" or \"at X\" then X is the location. If no location is specified \
    //    respond with \"Unknown\". Respond only in JSON."
    //);
    let chooser_prompt = format!(
        "Given the event details: \"{user_prompt}\", where is the location of the event? If the \
        event says \"go to X\" or \"at X\" then X is the location. Respond only in JSON."
    );
    dbg!(&chooser_prompt);
    let mut location =
        process::<Location>(&ollama, &model, &sys_prompt, &chooser_prompt, true).await?;
    location.cleanup();
    println!("final loc: {:?}", location);

    let chooser_prompt = format!(
        "Given the event details: \"{user_prompt}\", who is attending the event? If no people are \
        specified respond with []. Respond only in JSON."
    );
    dbg!(&chooser_prompt);
    let _where = process::<Names>(&ollama, &model, &sys_prompt, &chooser_prompt, true).await?;

    Ok(())
}

async fn process<S: Clone + Default + Debug + PartialEq + for<'a> Deserialize<'a> + JsonSchema>(
    o: &Ollama,
    model: &str,
    sys_prompt: &str,
    chooser_prompt: &str,
    verbose: bool,
) -> Result<S, Box<dyn std::error::Error>> {
    let mut final_resp: S = S::default();
    let mut accumulated_resp: Vec<(S, u8)> = Vec::new();
    'OUTER: for _ in 0..=20 {
        let format = FormatType::StructuredJson(Box::new(JsonStructure::new::<S>()));
        let res = {
            o.generate(
                GenerationRequest::new(model.to_string(), chooser_prompt)
                    .system(sys_prompt)
                    .format(format)
                    .think(true)
                    .options(ModelOptions::default().temperature(1.0)),
            )
            .await?
        };
        let resp: S = serde_json::from_str(&res.response)?;
        if verbose {
            dbg!(&resp);
        }
        // add the response to the accumulated response, and increment the count
        let mut found = false;
        for (r, c) in accumulated_resp.iter_mut() {
            if r == &resp {
                found = true;
                *c += 1;
                if *c >= 3 {
                    final_resp = resp.clone();
                    break 'OUTER;
                }
            }
        }
        if !found {
            accumulated_resp.push((resp.clone(), 1));
        }
    }
    if verbose {
        println!("final resp: {:?}", final_resp);
    }
    Ok(final_resp)
}

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

//fn add_to_calendar_abs(a: AddToCalendarAbsolute) {
//    dbg!(a);
//}

// add to calendar relative

//fn add_to_calendar_rel(a: AddToCalendarRelative) {
//    let mut dt = UtcDateTime::now();

//match (a.in_days, a.in_weeks) {
//    (Some(days), Some(weeks)) => {
//        // for silly llm mistakes where is provides both in_days and in_weeks
//        if weeks * 7 == days {
//            dt = dt.checked_add(Duration::days(days)).unwrap();
//        }
//    }
//    (Some(days), None) => {
//        dt = dt.checked_add(Duration::days(days)).unwrap();
//    }
//    (None, Some(weeks)) => {
//        dt = dt.checked_add(Duration::weeks(weeks)).unwrap();
//    }
//    (None, None) => {}
//}

// correct the day if necessary
//if let Some(on_day) = a.on_day {
//    if let Ok(on_day) = Weekday::from_str(&on_day) {
//        let existing_day = dt.weekday();
//        if existing_day != on_day {
//            let days_to_add = on_day.number_days_from_sunday() as i8
//                - existing_day.number_days_from_sunday() as i8;
//            dt = dt.checked_add(Duration::days(days_to_add as i64)).unwrap();
//        }
//    }
//    if on_day == "Tomorrow" {
//        dt = UtcDateTime::now();
//        dt = dt.checked_add(Duration::days(1)).unwrap();
//    }
//    println!("ON_DAY: {on_day}");
//}

// parse the time from the user input
//let format = format_description!("[hour]:[minute]");
//let dt = if let Ok(time_) = Time::parse(a.time, &format) {
//    // replace the time with the parsed time
//    dt.replace_time(time_)
//} else {
//    dt
//};

//    let date_format = format_description!("[year]-[month]-[day]");
//    let entry_info = json!(dt.format(date_format).unwrap());
//    dbg!(&entry_info);
//}
