use anyhow::Result;
use mlua::Lua;

pub struct ScriptEnv {
    lua: Lua,
}

fn init_lua_environment(lua: &Lua) -> Result<()> {
    let hello_world = lua.create_function(|_, _: ()| {
        println!("Hello, world!");
        Ok(())
    })?;

    let videoland = lua.create_table()?;
    videoland.set("hello", hello_world)?;

    lua.globals().set("videoland", videoland)?;

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
}
