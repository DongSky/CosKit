use coskit::{
    local_edit::{prepare_node, raster_edit, EditRequest},
    models::{EditNode, Layer},
};
use image::{DynamicImage, Rgba, RgbaImage};
use serde_json::json;

fn req(op: &str) -> EditRequest {
    EditRequest {
        operation: op.into(),
        layer_id: String::new(),
        params: json!({}),
        points: vec![],
        mask: None,
    }
}

#[test]
fn ai_mask_composite_preserves_protected_transparency() {
    let original = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 1, Rgba([10, 20, 30, 80])));
    let result = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 1, Rgba([40, 50, 60, 255])));
    let mut mask = RgbaImage::from_pixel(2, 1, Rgba([255, 255, 255, 255]));
    mask.put_pixel(1, 0, Rgba([0, 0, 0, 0]));
    let output = coskit::image_utils::composite_with_mask(
        &original,
        &result,
        &DynamicImage::ImageRgba8(mask),
    )
    .to_rgba8();
    assert_eq!(output.get_pixel(0, 0), &Rgba([10, 20, 30, 80]));
    assert_eq!(output.get_pixel(1, 0), &Rgba([40, 50, 60, 255]));
}
fn image() -> RgbaImage {
    RgbaImage::from_fn(32, 24, |x, y| {
        Rgba([(x * 5 + 20) as u8, (y * 6 + 10) as u8, 80, 180])
    })
}
fn mask() -> String {
    let m = RgbaImage::from_fn(32, 24, |x, y| {
        Rgba([
            255,
            255,
            255,
            if x > 8 && x < 16 && y > 6 && y < 14 {
                0
            } else {
                255
            },
        ])
    });
    coskit::image_utils::bytes_to_base64(
        &coskit::image_utils::image_to_png_bytes(&DynamicImage::ImageRgba8(m)).unwrap(),
    )
}
#[test]
fn selection_preserves_exact_rgba_outside() {
    let input = image();
    let mut r = req("adjust");
    r.params = json!({"brightness":25});
    r.mask = Some(mask());
    let output = raster_edit(&input, &r).unwrap();
    assert_ne!(input.get_pixel(10, 10), output.get_pixel(10, 10));
    for y in 0..24 {
        for x in 0..32 {
            if !(x > 8 && x < 16 && y > 6 && y < 14) {
                assert_eq!(input.get_pixel(x, y), output.get_pixel(x, y));
            }
        }
    }
}
#[test]
fn brush_interpolates_continuously_and_preserves_original() {
    let input = image();
    let mut r = req("brush");
    r.params = json!({"size":4,"hardness":1,"color":"#ff0000"});
    r.points = vec![[3., 10.], [28., 10.]];
    let output = raster_edit(&input, &r).unwrap();
    for x in 3..28 {
        assert_eq!(output.get_pixel(x, 10), &Rgba([255, 0, 0, 255]));
    }
    assert_eq!(input.get_pixel(0, 0), output.get_pixel(0, 0));
}
#[test]
fn eraser_only_reduces_alpha() {
    let input = image();
    let mut r = req("erase");
    r.params = json!({"size":6,"hardness":1});
    r.points = vec![[12., 12.]];
    let output = raster_edit(&input, &r).unwrap();
    assert_eq!(output.get_pixel(12, 12)[3], 0);
    assert_eq!(
        &output.get_pixel(12, 12).0[..3],
        &input.get_pixel(12, 12).0[..3]
    );
}
#[test]
fn clone_reads_frozen_source() {
    let input = image();
    let mut r = req("clone");
    r.params = json!({"size":4,"hardness":1,"source_x":4,"source_y":4});
    r.points = vec![[20., 16.]];
    let output = raster_edit(&input, &r).unwrap();
    assert_eq!(output.get_pixel(20, 16), input.get_pixel(4, 4));
}
#[test]
fn reject_invalid_requests_without_panics() {
    for (op, p) in [
        ("blur", json!({"radius":-1})),
        ("adjust", json!({"exposure":99})),
        ("brush", json!({"color":"#你好啊"})),
        ("resize", json!({"width":0})),
    ] {
        let mut r = req(op);
        r.params = p;
        r.points = vec![[5., 5.]];
        assert!(raster_edit(&image(), &r).is_err());
    }
    let mut r = req("brush");
    r.points = vec![[f32::NAN, 0.]];
    assert!(raster_edit(&image(), &r).is_err());
    assert!(raster_edit(&image(), &req("inpaint")).is_err());
}
#[test]
fn content_aware_fill_preserves_unselected_alpha() {
    let mut input = RgbaImage::from_pixel(32, 24, Rgba([80, 120, 160, 180]));
    input.put_pixel(12, 10, Rgba([255, 0, 0, 180]));
    let mut r = req("inpaint");
    r.mask = Some(mask());
    let output = raster_edit(&input, &r).unwrap();
    assert_eq!(output.get_pixel(0, 0), input.get_pixel(0, 0));
    assert_eq!(output.get_pixel(12, 10)[3], 180);
    assert!(output.get_pixel(12, 10)[0] < 150);
}
#[test]
fn dodge_and_burn_keep_alpha() {
    for op in ["dodge", "burn", "sponge"] {
        let mut r = req(op);
        r.points = vec![[12., 12.]];
        let input = image();
        let output = raster_edit(&input, &r).unwrap();
        assert_eq!(output.get_pixel(12, 12)[3], 180);
        assert_ne!(output, input);
    }
}

struct Fixture {
    dir: std::path::PathBuf,
    parent: EditNode,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("coskit-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("base.png");
        image().save(&path).unwrap();
        let mut parent = EditNode::new("root".into(), None);
        parent.status = "done".into();
        parent.image_path = path.to_string_lossy().into();
        parent.layers = vec![Layer::new_base(parent.image_path.clone())];
        Self { dir, parent }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
#[test]
fn geometry_transforms_every_layer_and_keeps_parent_files() {
    let f = Fixture::new();
    let bytes = std::fs::read(&f.parent.image_path).unwrap();
    let dup = prepare_node(&f.parent, &req("layer_duplicate"), &f.dir).unwrap();
    let mut r = req("crop");
    r.params = json!({"x":2,"y":3,"width":20,"height":12});
    let child = prepare_node(&dup, &r, &f.dir).unwrap();
    assert_eq!(child.metadata["canvas_size"], json!([20, 12]));
    for l in child.layers {
        assert_eq!(image::image_dimensions(l.image_path).unwrap(), (20, 12));
    }
    assert_eq!(std::fs::read(&f.parent.image_path).unwrap(), bytes);
}
#[test]
fn locked_layers_and_invalid_crop_leave_parent_unchanged() {
    let mut f = Fixture::new();
    f.parent.layers[0].locked = true;
    let mut r = req("adjust");
    r.params = json!({"brightness":10});
    assert!(prepare_node(&f.parent, &r, &f.dir).is_err());
    r = req("layer_props");
    r.params = json!({"locked":false});
    let child = prepare_node(&f.parent, &r, &f.dir).unwrap();
    assert!(f.parent.layers[0].locked);
    assert!(!child.layers[0].locked);
    r = req("crop");
    r.params = json!({"x":31,"width":5});
    assert!(prepare_node(&child, &r, &f.dir).is_err());
}
#[test]
fn layer_operations_are_independent_snapshots() {
    let f = Fixture::new();
    let added = prepare_node(&f.parent, &req("layer_new"), &f.dir).unwrap();
    assert_eq!(added.layers.len(), 2);
    assert_eq!(f.parent.layers.len(), 1);
    let mut r = req("layer_props");
    r.params = json!({"name":"paint","opacity":0.4,"blend_mode":"screen"});
    let renamed = prepare_node(&added, &r, &f.dir).unwrap();
    assert_eq!(renamed.layers[1].name, "paint");
    assert_eq!(added.layers[1].opacity, 1.);
    let deleted = prepare_node(&renamed, &req("layer_delete"), &f.dir).unwrap();
    assert_eq!(deleted.layers.len(), 1);
    assert_eq!(renamed.layers.len(), 2);
    assert!(prepare_node(&f.parent, &req("layer_delete"), &f.dir).is_err());
}

#[test]
fn history_branches_persist_and_failed_commit_is_atomic() {
    let f = Fixture::new();
    coskit::settings::set_app_data_dir(f.dir.clone());
    let mut session = coskit::models::Session::new("history".into(), f.parent.id.clone(), (32, 24));
    session.nodes.insert(f.parent.id.clone(), f.parent.clone());
    session.active_path = vec![f.parent.id.clone()];
    let first = prepare_node(&f.parent, &req("layer_new"), &f.dir).unwrap();
    let first_id = first.id.clone();
    coskit::local_edit::commit_node(&mut session, first).unwrap();
    let second = prepare_node(&f.parent, &req("layer_duplicate"), &f.dir).unwrap();
    let second_id = second.id.clone();
    coskit::local_edit::commit_node(&mut session, second).unwrap();
    assert_eq!(session.nodes["root"].children, vec![first_id, second_id]);
    coskit::engine::goto_node(&mut session, "root");
    assert_eq!(session.active_path, vec!["root"]);
    let loaded = coskit::engine::load_session("history").unwrap();
    assert_eq!(loaded.nodes.len(), 3);
    assert_eq!(loaded.active_path, vec!["root"]);
    let before = serde_json::to_value(&session).unwrap();
    std::fs::create_dir(f.dir.join("history/session.json.tmp")).unwrap();
    let failing = prepare_node(&f.parent, &req("layer_new"), &f.dir).unwrap();
    assert!(coskit::local_edit::commit_node(&mut session, failing).is_err());
    assert_eq!(serde_json::to_value(&session).unwrap(), before);
}
