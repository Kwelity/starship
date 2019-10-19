use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use dirs::home_dir;

use super::{Context, Module, RootModuleConfig};

use crate::configs::aws::{AwsConfig, AwsItems};

fn get_aws_region_from_config(aws_profile: Option<&str>) -> Option<String> {
    let config_location = env::var("AWS_CONFIG_FILE")
        .ok()
        .and_then(|path| PathBuf::from_str(&path).ok())
        .or_else(|| {
            let mut home = home_dir()?;
            home.push(".aws/config");
            Some(home)
        })?;

    let file = File::open(&config_location).ok()?;
    let reader = BufReader::new(file);
    let lines = reader.lines().filter_map(Result::ok);

    let region_line = if let Some(ref aws_profile) = aws_profile {
        lines
            .skip_while(|line| line != &format!("[profile {}]", aws_profile))
            .skip(1)
            .take_while(|line| !line.starts_with('['))
            .find(|line| line.starts_with("region"))
    } else {
        lines
            .skip_while(|line| line != "[default]")
            .skip(1)
            .take_while(|line| !line.starts_with('['))
            .find(|line| line.starts_with("region"))
    }?;

    let region = region_line.split('=').nth(1)?;
    let region = region.trim();

    Some(region.to_string())
}

type Profile = String;
type Region = String;

fn get_aws_profile_and_region() -> (Option<Profile>, Option<Region>) {
    match (
        env::var("AWS_PROFILE").ok(),
        env::var("AWS_REGION").ok(),
        env::var("AWS_DEFAULT_REGION").ok(),
    ) {
        (Some(p), Some(_), Some(dr)) => (Some(p), Some(dr)),
        (Some(p), Some(r), None) => (Some(p), Some(r)),
        (None, Some(r), None) => (None, Some(r)),
        (Some(p), None, Some(dr)) => (Some(p), Some(dr)),
        (Some(ref p), None, None) => (Some(p.to_owned()), get_aws_region_from_config(Some(p))),
        (None, None, Some(dr)) => (None, Some(dr)),
        (None, Some(_), Some(dr)) => (None, Some(dr)),
        (None, None, None) => (None, get_aws_region_from_config(None)),
    }
}

fn get_aws_region() -> Option<Region> {
    match (
        env::var("AWS_REGION").ok(),
        env::var("AWS_DEFAULT_REGION").ok(),
    ) {
        (Some(r), None) => Some(r),
        (None, Some(dr)) => Some(dr),
        (Some(_), Some(dr)) => Some(dr),
        (None, None) => get_aws_region_from_config(None),
    }
}

pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    const AWS_PREFIX: &str = "on ";

    let mut module = context.new_module("aws");
    let config: AwsConfig = AwsConfig::try_load(module.config);

    module.set_style(config.style);

    module.get_prefix().set_value(AWS_PREFIX);

    module.create_segment("symbol", &config.symbol);
    match config.displayed_items {
        AwsItems::All => {
            let (aws_profile, aws_region) = get_aws_profile_and_region();

            let aws_segment = match (&aws_profile, &aws_region) {
                (None, None) => return None,
                (Some(p), Some(r)) => format!("{}({})", p, r),
                (Some(p), None) => p.to_string(),
                (None, Some(r)) => r.to_string(),
            };
            module.create_segment("all", &config.region.with_value(&aws_segment));
        }
        AwsItems::Profile => {
            let aws_profile = env::var("AWS_PROFILE").ok().unwrap_or_default();

            module.create_segment("profile", &config.profile.with_value(&aws_profile));
        }
        AwsItems::Region => {
            let aws_region = get_aws_region().unwrap_or_default();

            module.create_segment("region", &config.region.with_value(&aws_region));
        }
    };

    Some(module)
}
