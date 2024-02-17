use ahash::AHashMap;
use uuid::Uuid;
use videoland_rhi as rhi;

pub struct ResourceContainer {
    textures: AHashMap<Uuid, rhi::Texture>,
}

impl ResourceContainer {
    pub fn new() -> Self {
        Self {
            textures: AHashMap::new(),
        }
    }

    pub fn read(&self, id: Uuid) -> Uuid {
        id
    }
}

struct PreparedPass {
    reads: Vec<Uuid>,
    writes: Vec<Uuid>,
    pass: Box<dyn Pass>,
}

pub struct FrameGraph<'a> {
    rc: &'a ResourceContainer,
    passes: Vec<PreparedPass>,
    screen: Uuid,
}

impl<'a> FrameGraph<'a> {
    pub fn new(rc: &'a ResourceContainer, screen: Uuid) -> Self {
        Self {
            rc,
            passes: Vec::new(),
            screen,
        }
    }

    pub fn add<P: Pass>(&mut self, mut pass: P) {
        let mut fetch = Fetch::new(&self.rc, self.screen);
        pass.fetch(&mut fetch);
        self.passes.push(PreparedPass {
            pass: Box::new(pass),
            reads: fetch.reads,
            writes: fetch.writes,
        });
    }

    pub fn execute(&mut self, rctx: RenderContext) {
        for pass in &mut self.passes {
            pass.pass.execute(&self.rc, &rctx);
        }
    }
}

pub struct Fetch<'a> {
    rc: &'a ResourceContainer,
    reads: Vec<Uuid>,
    writes: Vec<Uuid>,
    screen: Uuid,
}

impl<'a> Fetch<'a> {
    fn new(rc: &'a ResourceContainer, screen: Uuid) -> Self {
        Self {
            rc,
            reads: Vec::new(),
            writes: Vec::new(),
            screen,
        }
    }

    pub fn read(&mut self, id: Uuid) -> Uuid {
        self.reads.push(id);
        id
    }

    pub fn write(&mut self, id: Uuid) -> Uuid {
        self.writes.push(id);
        id
    }

    pub fn screen(&self) -> Uuid {
        self.screen
    }

    pub fn write_allocate(&mut self) -> Uuid {
        unimplemented!()
    }
}

pub trait Pass: 'static {
    fn fetch(&mut self, f: &mut Fetch);
    fn execute(&mut self, rc: &ResourceContainer, rctx: &RenderContext);
}

pub struct RenderContext {
    pub cmd: rhi::CommandBuffer,
}

#[derive(Default)]
pub struct EguiPass {
    output: Uuid,
}

impl Pass for EguiPass {
    fn fetch(&mut self, f: &mut Fetch) {
        self.output = f.write(f.screen());
    }

    fn execute(&mut self, rc: &ResourceContainer, rctx: &RenderContext) {}
}
