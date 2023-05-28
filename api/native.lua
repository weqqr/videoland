---@meta

-- This file contains type hints for APIs defined in native code. None of the
-- code below is ever executed; it's there just for type checking and code
-- completion in IDEs.

videoland = {}
videoland.lm = {}

function videoland.hello() end

---@class Level
Level = {}

---@return Level
function videoland.lm.new_level() end

---@param level Level
function videoland.lm.set_root(level) end

---@param level Level
function videoland.lm.unload(level) end
