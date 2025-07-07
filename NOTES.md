- TUI Calendar 
  - MINI TUI Overview
     - basically the notifications window but with some intractability
  - Month View
  - Mini Editor
     - manually adding a new entry
  - New terminal notifications bar up top
     - Every time you open up a new terminal tab at the top I could run a
       little script which would display a calendar preview as well as 
       perform any calendar time based actions
     - potentially annoying and should only run on new tabs if the preview
       hasn't been generated in the last (X-hours) 
  - scripts
    - calendar entries should be able to run little scripts either on
      notifications or on their calendar time (mediation timer, bot to perform
      a task for you)

  - probably build to sync with SimpleCalendar https://github.com/SimpleMobileTools/Simple-Calendar
    - OR maybe https://github.com/FossifyOrg/Calendar?
    - (ProtonCalendar doesn't have an API)

  - send notifications with https://github.com/hoodie/notify-rust
    - would require a daemon of some kind... annoying to setup / install
      - https://docs.rs/daemonize/latest/daemonize/
    - maybe not, def optional, not super interested personally I think
  - HLM (Heuristic Language Model) integration 
    "tcal sports team general meeting 5pm next friday" 
       - get llamda to get determine the date based on the current date and the
         text
       - determine reoccurance of entry
       - calendar color / which calendar to add it to (or default calendar)
       - determine location and time
       - determine other guests and email them invites (optional) 

- Heuristic Language Model (HLM)
   - a library AND a binary
   - binary: 
     - init config (hlm_gen_config.toml)
     - generate examples long list (hlm_examples.toml)
     - generate correct output for each example 
        - (edit hlm_examples.toml adding answers where were NONE before)
     - generate long-list of templates for each example (hlm_examples.toml)
        - add the template within each example in order to draw the connection
          as to where that template came from.
     - create HLM files (hlms/my-hlm-[hash].hlm) 
        - each hlm file will have success rates and failures baked in
     - list hlm files by success rate
   - based purely on heuristics in rust

Generate 10 example requests of how a variety of different people might request
an entry to be added to their own personal calendar. Respond in a json array. For
example: 
[
"sports team general meeting 5pm next friday",
....
]

      - each match could be in the form: 
      - "[phrase] [time] [date]" could match "sports team general meeting 5pm next friday"
      - "add to my [name] calendar: [phase] on [date] at [name]" 
      - "create a new calendar named [single-word]"
   - calculated the similarity of words if no exact matches found
     - https://github.com/life4/textdistance.rs
     - probably use "edit" based algo
     - probably user only the fast algo... I guess the user could specify
       - probably jaro_winkler or levenshtein
   - Bulk generation of what you want your HLM to look like based on actual LLMs
      - ollama, open-ai with async-openai lib.
      - use LLM to generate a long list of example inputs.
      - use LLM to generate the correct output parsing for each input. Save this
        list for manual editing/verification.
        - Populate a list of the CORRECT parsing for each example
           - can be done with an LLM
           - Eg. sports team general meeting 5pm next friday 
               - CORRECT [phrase] = "sports team general meeting"
               - CORRECT [time] = "5pm" 
               - CORRECT [date] = "next friday"
           - Output (TODO correct json) structure the same as async-openai
             output.
              ToolCalls {
                function name: add_to_calendar_relative
                parameters {
                  param: "sports team general meeting" "
                  time: "5pm"
                  date: "next friday"
                }
              }
      - for each example input use LLM to create an input template
        automatically. 
         - add this to the long list of input templates (written in examples)
         - Then we can actually test each of those example heuristics out
           (removing heuristic calls which don't produce the previously
           specified correct output).
      - For each example 
       - verification of heuristics:
        - maintain cache of all LLM evaluations which verify or deny each example
          running through a particular heuristic. This way we can limit LLM
          calls and just retrieve from the cache. 
           - we should persist this cache in a file somewhere
        - Once we have a long list of the heuristics AND a long list of the
          examples we can determine the ordering of the match-phrases
        - Each of the match phrases should be assigned an ID could just be hash
          or a counter. 
        - Start with a specific ordering of the match-phrases (could be random)
           - could retry this process a few times with different random starting
             phrases in order to attempt to find better minima. 
           - For each ordering BEFORE we perform any re-arrangements we should
             attempt to log the %-success for each ordering of the phrases... 
              - DO NOT attempt to find subsequent GOOD matches if a BAD match is
                first found in the list, this information is just for populating
                the %-success.
              - log all failures (which phrases fail) with the success of the
                ordering.
              - ALSO log any of the heuristics which were never used!
        - Go through and attempt to match each example phrase.
           - For each example phrase which matches, perform a verification as to
             if the match seems appropriate. 
             - if GOOD then the order is OKAY
             - if BAD then continue through the list until a match is found
               which matches and is verified to as GOOD. Then move the GOOD
               match to a position in front of the first BAD match. 
             - continue through to the NEXT example (even though this shift will
               have changed all earlier examples).
             - Once all the examples have been gone through run a "%-success"
               analysis and log this information with the current ordering AND
               with the current match proceedure.
             - if not 100% success then go through from the start of the list of
               phrases from the start and attempt to minimize the %failure. 
                - if can't minimize further in say 10 subsequent iterations
                  through all the examples then simply accept the best %success
                  (or multiple if there are ties) as the best outputs.
            
