use std::{
    collections::HashMap,
    env::Args,
    fmt::Debug,
    hash::Hash,
    iter::{Peekable, Skip},
    str::FromStr,
};

pub trait ToFlagConfiguration {
    fn to_config(&self) -> FlagConfiguration;
}

#[derive(Debug)]
pub struct Command<T>
where
    T: FromStr + PartialEq,
{
    pub value: T,
}

pub fn read_command<T>(args: &mut Peekable<Skip<Args>>) -> Result<Command<T>, String>
where
    T: FromStr + PartialEq,
{
    let Some(arg) = args.next() else {
        return Err(String::new());
    };
    let value = T::from_str(&arg).map_err(|_| String::new())?;

    Ok(Command { value })
}

#[derive(Debug)]
pub struct Flag {
    pub value: String,
}

impl Flag {
    fn value<T>(&self) -> T
    where
        T: FromStr,
        T::Err: Debug,
    {
        T::from_str(&self.value).unwrap()
    }
}

#[derive(Debug)]
pub enum FlagPresence {
    Optional,
    Required,
}

#[derive(Debug)]
pub struct FlagConfiguration {
    pub long_name: String,
    pub short_name: Option<String>,
    pub description: String,
    pub presence: FlagPresence,
}

impl FlagConfiguration {
    pub fn required(
        long_name: &str,
        short_name: Option<&str>,
        description: &str,
    ) -> FlagConfiguration {
        FlagConfiguration {
            long_name: String::from(long_name),
            short_name: short_name.map(|short_name| String::from(short_name)),
            description: String::from(description),
            presence: FlagPresence::Required,
        }
    }

    pub fn optional(
        long_name: &str,
        short_name: Option<&str>,
        description: &str,
    ) -> FlagConfiguration {
        FlagConfiguration {
            long_name: String::from(long_name),
            short_name: short_name.map(|short_name| String::from(short_name)),
            description: String::from(description),
            presence: FlagPresence::Optional,
        }
    }
}

pub fn read_flags<T>(args: &mut Peekable<Skip<Args>>, flags: &Vec<T>) -> FlagMap<T>
where
    T: PartialEq + Eq + Hash + ToFlagConfiguration + Clone + Debug,
{
    let mut flag_map: FlagMap<T> = FlagMap::new();
    let mut flag_configuration_map: HashMap<&T, FlagConfiguration> = HashMap::default();
    let mut long_name_map: HashMap<&str, &T> = HashMap::default();
    let mut short_name_map: HashMap<&str, &T> = HashMap::default();

    for flag in flags {
        flag_configuration_map.insert(flag, flag.to_config());
    }

    for flag in flags {
        let configuration = &flag_configuration_map[flag];
        long_name_map.insert(&configuration.long_name, flag);

        if let Some(short_name) = &configuration.short_name {
            short_name_map.insert(short_name, flag);
        }
    }

    while let Some(flag_name) = args.peek() {
        if !flag_name.starts_with("-") {
            break;
        }
        let flag_name = args.next().unwrap();
        match flag_name {
            _ if flag_name.starts_with("--") => {
                let Some(flag_key) = long_name_map.get(flag_name.replace("--", "").as_str()) else {
                    panic!("Should get key");
                };
                flag_map.insert(
                    flag_key,
                    Flag {
                        value: args.next().unwrap(),
                    },
                );
            }
            _ if flag_name.starts_with("-") => {
                for short_name in flag_name.chars().skip(1) {
                    let Some(flag_key) = short_name_map.get(short_name.to_string().as_str()) else {
                        panic!("Should get key");
                    };
                    flag_map.insert(
                        flag_key,
                        Flag {
                            value: args.next().unwrap(),
                        },
                    );
                }
            }
            _ => break,
        }
    }

    for flag in flags {
        let configuration = &flag_configuration_map[flag];

        if matches!(configuration.presence, FlagPresence::Required) && !flag_map.has(flag) {
            panic!("Flag is required");
        }
    }

    flag_map
}

#[derive(Debug)]
pub struct FlagMap<T>
where
    T: PartialEq + Eq + Hash + Clone + Debug,
{
    flag_map: HashMap<T, Flag>,
}

impl<T> FlagMap<T>
where
    T: PartialEq + Eq + Hash + Clone + Debug,
{
    pub fn new() -> FlagMap<T> {
        FlagMap {
            flag_map: HashMap::new(),
        }
    }

    pub fn has(&self, key: &T) -> bool {
        self.flag_map.contains_key(key)
    }

    pub fn insert(&mut self, key: &T, flag: Flag) {
        self.flag_map.insert(key.clone(), flag);
    }

    pub fn get<V>(&self, key: T) -> V
    where
        V: FromStr,
        V::Err: Debug,
    {
        self.flag_map.get(&key).unwrap().value()
    }

    pub fn get_optional<V>(&self, key: T) -> Option<V>
    where
        V: FromStr,
        V::Err: Debug,
    {
        self.flag_map
            .get(&key)
            .map_or(None, |flag| Some(flag.value()))
    }
}
