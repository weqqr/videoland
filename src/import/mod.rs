use anyhow::Result;
use std::io::Cursor;

use crate::res::{Vertex, Model, Mesh};

pub fn obj(data: Vec<u8>) -> Result<Model> {
    let reader = Cursor::new(data);
    let obj = obj::ObjData::load_buf(reader)?;

    let mut model = Model::new();

    let vertex = |indices: obj::IndexTuple| Vertex {
        position: obj.position[indices.0].into(),
        normal: indices.2.map(|n| obj.normal[n]).unwrap_or([0.0; 3]).into(),
        texcoord: indices.1.map(|t| obj.texture[t]).unwrap_or([0.5; 2]).into(),
    };

    for group in obj.objects.iter().flat_map(|o| o.groups.iter()) {
        let mut mesh = Mesh::new();

        for poly in &group.polys {
            let base = poly.0[0];

            for i in 0..poly.0.len() - 2 {
                mesh.add_vertex(vertex(base));
                mesh.add_vertex(vertex(poly.0[i + 1]));
                mesh.add_vertex(vertex(poly.0[i + 2]));
            }
        }

        model.add_mesh(mesh);
    }

    Ok(model)
}
