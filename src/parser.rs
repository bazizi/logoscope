use anyhow::Result;
use async_std::io::ReadExt;
use std::path::PathBuf;

use log::info;

use lazy_static::lazy_static;
use regex::Regex;

pub type LogEntry = Vec<String>;

lazy_static! {
    static ref REGEX_PATTERNS : Vec<Regex> = vec![
        Regex::new(r#"^\s*[^\[]+\[(?P<date>\d{2}:\d{2}:\d{2}:\d+)\]:\s*(?P<log>.*)$"#).unwrap(),                                                        // Windows installer (MSI)
        Regex::new(r#"^\s*(?P<id>\d+)\s+\[(?P<date>[^\]]+)\]\s+PID:\s*(?P<pid>\d+)\s+TID:\s*(?P<tid>\d+)\s+(?P<level>\w+)\s+(?P<log>.*)$"#).unwrap(),   // EA app
        Regex::new(r#"^\s*\[[^\]]+\]\[(?P<date>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\]i\d+:\s*(?P<log>.*)$"#).unwrap(),                                  // EA app vc_redist
        Regex::new(r#"^\s*(?P<level>\w+)\s+(?P<date>\d{2}:\d{2}:\d{2}\s+\w+)\s+\(\s+\d+\)\s+(?P<tid>\d+)\s+(?P<log>.*)$"#).unwrap(),                    // EA app IGO
        Regex::new(r#"^\s*(?P<level>\w+)\s+(?P<date>\d{2}:\d{2}:\d{2}\s+\w+)\s+(?P<tid>\d+)\s+\s+(?P<log>.*)$"#).unwrap(),                              // EA app IGO Proxy
        Regex::new(r#"^\s*\[(?P<date>\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2})\]\s+(?P<log>.*)$"#).unwrap(),                                                // Steam
        Regex::new(r#"^\s*\[(?P<date>\d{4}\.\d{2}\.\d{2}-\d{2}\.\d{2}\.\d{2}:\d+)\][^\]]+\](?P<log>.*)$"#).unwrap(),                                    // Riot launcher (Valorant) + Epic games
        Regex::new(r#"^\s*\[(?P<date>[^:]+):(?P<level>\w+):[^\]]+\]\s*(?P<log>.*)$"#).unwrap(),                                                         // CEF
        Regex::new(r#"^(?P<date>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}Z)\s+(?P<level>\w+)\s+(?P<log>.*)$"#).unwrap(),                                // NodeJS
        Regex::new(r#"^(?P<date>\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2})\s+(?P<level>\w+)\s+(?P<log>.*)$"#).unwrap(),                                // dpkg log (Linux)
    ];
}

pub enum LogEntryIndices {
    FileName,
    // _ID,
    Date,
    // _PID,
    // _TID,
    Level,
    Log,
}

fn parse_log_vec(lines: &[&str], log_path: &PathBuf) -> Vec<Vec<String>> {
    let mut line_num = 0;
    let mut _session = 0;
    let mut log_entries = Vec::<Vec<String>>::new();

    while line_num < lines.len() {
        let mut log = String::new();
        let line = lines[line_num];
        if line.is_empty() {
            line_num += 1;
            continue;
        }

        let mut captures = None;
        for regex_pattern in REGEX_PATTERNS.iter() {
            let mut captures_iter = regex_pattern.captures_iter(line);

            if let Some(tmp) = captures_iter.next() {
                captures = Some(tmp);
                break;
            }
        }

        if captures.is_none() {
            info!("Error parsinig line: [{}]", line);
            line_num += 1;
            continue;
        }

        let captures = captures.unwrap();
        let _id = line_num;
        if captures.name("id").map_or("", |m| m.as_str()) == "0" {
            _session += 1;
        }
        let date = &captures.name("date").map_or("", |m| m.as_str());
        let _pid = &captures.name("pid").map_or("", |m| m.as_str());
        let _tid = &captures.name("tid").map_or("", |m| m.as_str());
        let level = &captures.name("level").map_or("", |m| m.as_str());
        log += &captures
            .name("log")
            .map_or("".to_owned(), |m| m.as_str().replace('\t', "    "));

        loop {
            // Deal with multiline log entries where only the 1st line matches the regex.
            // We append the next lines to the first line and show them as a single log entry
            if line_num >= lines.len() - 1 {
                break;
            }

            line_num += 1;
            let next_line = lines[line_num];

            let mut valid_captures = None;
            for regex_pattern in REGEX_PATTERNS.iter() {
                let mut captures = regex_pattern.captures_iter(next_line);

                if captures.next().is_some() {
                    valid_captures = Some(captures);
                    break;
                }
            }

            if valid_captures.is_none() {
                // Current line doesn't match any known formats so we assume it's a continuation of a multiline log entry
                log += next_line;
                continue;
            }

            // Current line is an actual log line (and not a continuation of a multiline log entry)
            // So we go back to the previous line and break (so that the current line will be processed as a separate entry)
            line_num -= 1;
            break;
        }

        log_entries.push(vec![
            log_path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string(),
            // id.to_string(),
            // _session.to_string(),
            date.to_string(),
            // pid.to_string(),
            // tid.to_string(),
            level.to_string(),
            log.to_string(),
        ]);

        line_num += 1;
    }

    info!(
        "found [{}] log entries in [{:?}]",
        log_entries.len(),
        log_path
    );

    log_entries
}

pub async fn parse_log_by_path_async(log_path: &PathBuf) -> Result<Vec<LogEntry>> {
    info!("Attempting to parse log file [{:?}]...", log_path);

    let mut contents = String::new();
    let lines = {
        let mut f = async_std::fs::File::open(log_path).await?;
        f.read_to_string(&mut contents).await?;
        contents.lines().collect::<Vec<&str>>()
    };

    Ok(parse_log_vec(&lines, log_path))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::parse_log_vec;

    fn verify_parsed_result(
        parsed_result: &Vec<Vec<String>>,
        num_expected_lines: usize,
        num_expected_cols: usize,
    ) {
        assert_eq!(parsed_result.len(), num_expected_lines);
        if num_expected_lines == 0 {
            return;
        }
        assert_eq!(parsed_result[0].len(), num_expected_cols);
    }

    #[test]
    fn test_msi_parse() {
        let log_lines = vec![
            "an invalid line",
            "[1E78:1CCC][2023-09-03T16:46:28]i001: Burn v3.8.1128.0, Windows v6.3 (Build 9600: Service Pack 0), path: C:\\Program Files (x86)\\Epic Games\\Launcher\\Portal\\SelfUpdateStaging\\Install\\Portal\\Extras\\Redist\\LauncherPrereqSetup_x64.exe, cmdline: '/quiet /log \"C:/Users/behna/AppData/Local/EpicGamesLauncher/Saved/Logs/SelfUpdatePrereqInstall.log\" -burn.unelevated BurnPipe.{728D72EE-4979-4A05-84E7-FCB1B3712CDD} {529A9DC5-F441-4AF4-ACBC-8809762531C3} 20096'",
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "another invalid line",
            "[1E78:1CCC][2023-09-03T16:46:28]i000: Setting string variable 'WixBundleLog' to value 'C:/Users/behna/AppData/Local/EpicGamesLauncher/Saved/Logs/SelfUpdatePrereqInstall.log'",
            "another invalid line",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 2, 4);
    }

    #[test]
    fn test_eaa_parse() {
        let log_lines = vec![
            "an invalid line",
            "720	[2023-12-26T06:41:43.537Z]	PID: 12196	TID: 13344	WARN    	(eax::components::contentLibrary::ContentLibraryComponent::Impl::refreshExternalEntitlements)	Skipping entitlement refresh due to missing a user session",
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "721	[2023-12-26T06:56:41.372Z]	PID: 12196	TID: 13344	INFO    	(eax::services::updater::UpdaterStateMachine::onUpdateCheckComplete)	Update is NOT available",
            "another invalid line",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 2, 4);
    }

    #[test]
    fn test_eaa_vc_redist_parse() {
        let log_lines = vec![
            "an invalid line",
            "MSI (s) (38:48) [20:10:58:904]: Note: 1: 1707 ",
            "MSI (s) (38:48) [20:10:58:904]: Product: Microsoft Visual C++ 2013 x64 Minimum Runtime - 12.0.40664 -- Installation completed successfully.",
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "MSI (s) (38:48) [20:10:58:905]: Windows Installer installed the product. Product Name: Microsoft Visual C++ 2013 x64 Minimum Runtime - 12.0.40664. Product Version: 12.0.40664. Product Language: 1033. Manufacturer: Microsoft Corporation. Installation success or error status: 0.",
            "another invalid line",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 3, 4);
    }

    #[test]
    fn test_eaa_igo_parse() {
        let log_lines = vec![
            "Process Information", // parser ignores this line
            "    PID: 1280",       // parser ignores this line
            "    EXE: C:\\Program Files\\Electronic Arts\\EA Desktop\\EA Desktop\\EADesktop.exe", // parser ignores this line
            "STARTED: Sat, Dec 23 2023 02:16:02 AM", // parser ignores this line
            "an invalid line",
            "WARN	02:16:02 AM (    0)	 8300         IGOTelemetry.cpp:   77		Unable to retrieve telemetry prod id",
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "WARN	02:16:02 AM (    0)	 8300         IGOTelemetry.cpp:   87		Unable to retrieve telemetry timestamp",
            "another invalid line",
            "WARN	02:16:02 AM (    3)	 8300              DllMain.cpp: 2191		isIGOSharedMemoryNew=1",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 3, 4);
    }

    #[test]
    fn test_eaa_igo_proxy_parse() {
        let log_lines = vec![
            "INFO	02:19:32 AM	16664	         Helpers.cpp:  628		Defaulf value for environment variable IGOLogDirPath is C:\\Users\\behna\\AppData\\Local\\Electronic Arts\\EA Desktop\\Logs",
            "Final value of the environment varialble IGOLogDirPath is C:\\Users\\behna\\AppData\\Local\\Electronic Arts\\EA Desktop\\Logs", // parser ignores this line
            "INFO	02:19:32 AM	16664	             DX9.cpp:   68		Looking up DX9 Offsets (64 bits)",
            "INFO	02:19:32 AM	16664	             DX9.cpp:  106		Using display format idx=0 (format=0x00000016 / mode.Format=0x00000016)",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 3, 4);
    }

    #[test]
    fn test_steam_parse() {
        let log_lines = vec![
            "[2023-12-10 23:18:08] Change number 21482018->21482152, apps: 0/113, packages: 0/7",
            "[2023-12-10 23:33:48] Change number 21482152->21482258, apps: 0/76, packages: 0/20",
            "an invalid line",
            "[2023-12-10 23:49:33] Change number 21482258->21482366, apps: 0/81, packages: 0/28",
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "[2023-12-23 13:44:05] Client version: 1702079146",
            "[2023-12-23 13:44:05] Packages changed: force all",
            "another invalid line",
            "[2023-12-23 13:44:05] Apps changed: force all",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 6, 4);
    }

    #[test]
    fn test_steam_parse2() {
        let log_lines = vec![
            "[2023-09-01 20:37:04] Client version: 1690583737",
            "[2023-09-01 20:37:04] Change number 0->20137690, apps: 0/0, packages: 0/0",
            "[2023-09-01 20:37:04] Requesting 1 apps, 142 packages (meta data, 1024 prev attempts, expected 6 KB)",
            "[2023-09-01 20:37:04] Downloaded 1 apps via HTTP, 13 KB (3 KB compressed)",
            "[2023-09-01 20:37:04] Requesting 0 apps, 142 packages (full data, 1024 prev attempts, expected 51 KB)",
            "[2023-09-01 20:37:04] UpdatesJob: finished OK, apps updated 1 (13 KB), packages updated 142 (44 KB)",
            "[2023-09-01 20:37:04] Requesting 428 apps, 0 packages (meta data, 1024 prev attempts, expected 20 KB)",
            "[2023-09-01 20:37:05] Requested 60 app access tokens, 50 received, 10 denied",
            "[2023-09-01 20:37:05] Updated SHAs with 50 new access tokens",
            "[2023-09-01 20:37:05] Downloaded 92 apps via HTTP, 4392 KB (1075 KB compressed)",
            "[2023-09-01 20:37:05] Requesting 336 apps, 0 packages (full data, 1024 prev attempts, expected 736 KB)",
        ];
        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 11, 4);
    }

    #[test]
    fn test_epic_parse() {
        let log_lines = vec![
            "LogConfig: Setting CVar [[s.FlushStreamingOnExit:1]]", // parser ignores this line
            "LogInit: Object subsystem initialized",                // parser ignores this line
            "LogConfig: Applying CVar settings from Section [ConsoleVariables] File [C:/Users/behna/AppData/Local/EpicGamesLauncher/Saved/Config/Windows/Engine.ini]", // parser ignores this line
            "[2023.10.08-05.40.07:182][  0]LogInit: Computer: DESKTOP-JQ0NCMI",
            "an invalid line",
            "[2023.10.08-05.40.07:182][  0]LogInit: CPU Page size=4096, Cores=4",
            "another invalid line",
            "[2023.10.08-05.40.07:182][  0]LogInit: High frequency timer resolution =10.000000 MHz",
            "another invalid line",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 3, 4);
    }

    #[test]
    fn test_cef_parse() {
        let log_lines = vec![
            "an invalid line",
            "[0901/211250.717:ERROR:adm_helpers.cc(62)] Failed to query stereo recording.",
            "", // left empty on purpose to ensure the parser can handle empty lines gracefully
            "an invalid line",
            "[0901/211250.738:WARNING:mediasession.cc(347)] Duplicate id found. Reassigning from 104 to 125",
            "an invalid line",
            "[0901/211250.798:WARNING:stunport.cc(384)] Jingle:Port[000002172D841260:data:1:0:local:Net[any:0:0:0:x:x:x:x:x/0:Unknown]]: StunPort: stun host lookup received error 0",
            "an invalid line",
            "[1013/215308.845:ERROR:adm_helpers.cc(62)] Failed to query stereo recording.",
            "an invalid line",
            "[1013/215308.863:WARNING:mediasession.cc(347)] Duplicate id found. Reassigning from 104 to 125",
            "an invalid line",
            "[1013/215308.921:WARNING:stunport.cc(384)] Jingle:Port[0000018F02628910:data:1:0:local:Net[any:0:0:0:x:x:x:x:x/0:Unknown]]: StunPort: stun host lookup received error 0",
            "an invalid line",
        ];

        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 6, 4);
    }

    #[test]
    fn test_dpkg_log_parse() {
        let log_lines = vec![
            "2023-11-22 21:36:20 startup archives install",
            "2023-11-22 21:36:20 install base-passwd:amd64 <none> 3.5.52build1",
            "2023-11-22 21:36:20 status half-installed base-passwd:amd64 3.5.52build1",
            "2023-11-22 21:36:20 status unpacked base-passwd:amd64 3.5.52build1",
        ];
        let parsed_result = parse_log_vec(&log_lines, &PathBuf::from(""));
        verify_parsed_result(&parsed_result, 4, 4);
    }
}
