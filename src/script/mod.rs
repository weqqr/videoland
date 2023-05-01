use anyhow::Result;
use mlua::{Function, Lua, Table};

const API_TABLE_NAME: &str = "videoland";

pub struct ScriptEnv {
    lua: Lua,
}

fn init_lua_environment(lua: &Lua) -> Result<()> {
    let hello_world = lua.create_function(|_, _: ()| {
        println!("Hello, world!");
        Ok(())
    })?;

    let api_table = lua.create_table()?;
    api_table.set("hello", hello_world)?;

    lua.globals().set(API_TABLE_NAME, api_table)?;

    Ok(())
}

impl ScriptEnv {
    pub fn new() -> Self {
        let lua = Lua::new();

        init_lua_environment(&lua).unwrap();

        Self { lua }
    }

    pub fn execute(&self, script: &[u8]) -> Result<()> {
        Ok(self.lua.load(script).exec()?)
    }

    fn api_table(&self) -> Table {
        self.lua.globals().get::<_, Table>(API_TABLE_NAME).unwrap()
    }

    pub fn on_event(&self, event_type: &str) {
        let on_event = self.api_table().get::<_, Function>("on_event").unwrap();
        on_event.call::<_, ()>((event_type,)).unwrap();
    }
}
