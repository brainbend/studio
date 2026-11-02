// A Web Lesson with a cropped picture and a cropped picture beside text, for the player tests: webcrop <out_dir>
use brainbend_doc::lesson::*;
use brainbend_doc::*;

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let mut p = Project::new_web_lesson("Crops", ProjectKind::WebLesson);
    p.assets.insert("pic".into(), Asset { name: "pic.png".into(), mime: "image/png".into(), bytes: vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 218, 99, 252, 207, 192, 80, 15, 0, 4, 133, 1, 128, 132, 169, 140, 33, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130], file: None });
    let l = p.lesson.as_mut().unwrap();
    l.sections = vec![Section { id: "s".into(), title: "Pictures".into(), summary: String::new(), blocks: vec![
        Block { id: "a".into(), kind: BlockKind::Image { asset: Some("pic".into()), alt: "Cropped".into(), caption: String::new(), width: ImageWidth::Full, decorative: false, crop: Some([0.25, 0.0, 0.25, 0.0]) } },
        Block { id: "b".into(), kind: BlockKind::ImageText { asset: Some("pic".into()), alt: "Beside".into(), text: "Words".into(), image_right: true, crop: Some([0.0, 0.1, 0.0, 0.4]) } },
        Block { id: "c".into(), kind: BlockKind::Image { asset: Some("pic".into()), alt: "Whole".into(), caption: String::new(), width: ImageWidth::Full, decorative: false, crop: None } },
    ] }];
    brainbend_publish_html::export(&p, std::path::Path::new(&out)).unwrap();
}
