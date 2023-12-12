use mlua::{Function, Lua, LuaOptions, StdLib, Table};

pub struct ScriptRuntime {
    lua: Lua,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        let lua = Lua::new_with(StdLib::ALL, LuaOptions::default()).unwrap();

        // FIXME: this is very not good
        std::env::set_var("LUAU_PATH", "data/script/?.luau;data/script/?.lua");

        let op = lua.create_table().unwrap();
        lua.globals().set("op", op).unwrap();

        let native = lua.create_table().unwrap();
        native
            .set("on_start_callbacks", lua.create_table().unwrap())
            .unwrap();
        lua.globals().set("native", native).unwrap();

        let init = std::fs::read_to_string("data/script/init.lua").unwrap();
        lua.load(init).set_name("@init.lua").exec().unwrap();

        Self { lua }
    }

    pub fn call_on_start(&self) {
        let native: Table = self.lua.globals().get("native").unwrap();
        let on_start: Table = native.get("on_start_callbacks").unwrap();

        for pair in on_start.pairs::<usize, Function>() {
            let pair = pair.unwrap();

            pair.1.call::<_, ()>(()).unwrap();
        }
    }
}
