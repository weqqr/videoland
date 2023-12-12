---@meta

---@alias Id string

---@class Scene
Scene = {}

---Returns a scene node
---@param id Id
---@return SceneNode
function Scene:get_node(id) end

function Scene:add_node(id, object) end

---@class SceneNode
SceneNode = {}

---Returns node name
---@return string
function SceneNode:name() end
