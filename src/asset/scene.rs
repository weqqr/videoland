use crate::scene::{Scene, Spatial};

#[derive(serde::Serialize, serde::Deserialize)]
struct SceneData {
    nodes: Vec<Spatial>,
}

pub fn import_scenejson(data: &[u8]) -> Scene {
    let sc: SceneData = serde_json::from_slice(data).unwrap();
    let mut scene = Scene::new();

    for node in sc.nodes {
        scene.add_node(node);
    }

    scene
}

pub fn export_scenejson(scene: &Scene) -> Vec<u8> {
    serde_json::to_vec(&SceneData {
        nodes: scene.nodes().to_vec(),
    }).unwrap()
}
