//! Builds small presentations in memory and checks what the importer makes of them.
use std::io::{Cursor, Write};
use brainbend_doc::*;

const NS: &str = r#"xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#;
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn rels(items: &[(&str, &str, &str)]) -> String {
    let body: String = items.iter().map(|(id, ty, target)| format!(r#"<Relationship Id="{id}" Type="{REL}/{ty}" Target="{target}"/>"#)).collect();
    format!(r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>"#)
}

fn theme() -> String {
    r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Test"><a:themeElements>
<a:clrScheme name="t"><a:dk1><a:srgbClr val="111111"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="224466"/></a:dk2><a:lt2><a:srgbClr val="EEEEEE"/></a:lt2>
<a:accent1><a:srgbClr val="FF0000"/></a:accent1><a:accent2><a:srgbClr val="00FF00"/></a:accent2><a:accent3><a:srgbClr val="0000FF"/></a:accent3><a:accent4><a:srgbClr val="FFFF00"/></a:accent4><a:accent5><a:srgbClr val="00FFFF"/></a:accent5><a:accent6><a:srgbClr val="FF00FF"/></a:accent6><a:hlink><a:srgbClr val="0000EE"/></a:hlink><a:folHlink><a:srgbClr val="551A8B"/></a:folHlink></a:clrScheme>
<a:fontScheme name="f"><a:majorFont><a:latin typeface="Montserrat"/></a:majorFont><a:minorFont><a:latin typeface="Open Sans"/></a:minorFont></a:fontScheme>
<a:fmtScheme name="x"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme>
</a:themeElements></a:theme>"#.to_string()
}

fn master() -> String {
    format!(r#"<p:sldMaster {NS}><p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="F0F0F0"/></a:solidFill></p:bgPr></p:bg><p:spTree>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="609600" y="274320"/><a:ext cx="10972800" cy="1143000"/></a:xfrm></p:spPr><p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:r><a:t>Click to edit</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="609600" y="1600200"/><a:ext cx="10972800" cy="4525963"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Click to edit</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="4" name="Logo bar"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="6400800"/><a:ext cx="12192000" cy="457200"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent3"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>
</p:spTree></p:cSld><p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
<p:sldLayoutIdLst><p:sldLayoutId id="1" r:id="rId1"/></p:sldLayoutIdLst>
<p:txStyles><p:titleStyle><a:lvl1pPr algn="l"><a:defRPr sz="4400" b="1"><a:solidFill><a:schemeClr val="tx2"/></a:solidFill><a:latin typeface="+mj-lt"/></a:defRPr></a:lvl1pPr></p:titleStyle>
<p:bodyStyle><a:lvl1pPr marL="228600" indent="-228600"><a:buChar char="•"/><a:defRPr sz="2800"><a:solidFill><a:schemeClr val="tx1"/></a:solidFill><a:latin typeface="+mn-lt"/></a:defRPr></a:lvl1pPr><a:lvl2pPr><a:buChar char="–"/><a:defRPr sz="2400"/></a:lvl2pPr></p:bodyStyle></p:txStyles></p:sldMaster>"#)
}

fn layout() -> String {
    // Title moves and is recolored; a decorative circle is added; the body is inherited as is.
    format!(r#"<p:sldLayout {NS}><p:cSld name="Title and Content"><p:spTree>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="1219200" y="609600"/><a:ext cx="9144000" cy="914400"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr algn="ctr"><a:defRPr sz="3600"/></a:lvl1pPr></a:lstStyle><a:p><a:r><a:t>Title</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Body"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Text</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="5" name="Dot"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="11000000" y="100000"/><a:ext cx="600000" cy="600000"/></a:xfrm><a:prstGeom prst="ellipse"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="FF8800"/></a:solidFill></p:spPr></p:sp>
</p:spTree></p:cSld></p:sldLayout>"#)
}

fn slide1() -> String {
    format!(r#"<p:sld {NS}><p:cSld><p:spTree>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title 1"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Hello world</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Content"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/>
<a:p><a:r><a:t>First </a:t></a:r><a:r><a:rPr b="1"><a:solidFill><a:srgbClr val="CC0000"/></a:solidFill></a:rPr><a:t>bold red</a:t></a:r></a:p>
<a:p><a:pPr lvl="1"/><a:r><a:t>Second level</a:t></a:r></a:p></p:txBody></p:sp>
<p:grpSp><p:nvGrpSpPr><p:cNvPr id="10" name="Group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="1000000" y="1000000"/><a:ext cx="2000000" cy="1000000"/><a:chOff x="0" y="0"/><a:chExt cx="1000000" cy="500000"/></a:xfrm></p:grpSpPr>
<p:sp><p:nvSpPr><p:cNvPr id="11" name="Inner"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="500000" y="250000"/><a:ext cx="500000" cy="250000"/></a:xfrm><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val="accent2"/></a:solidFill><a:ln w="25400"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></p:spPr><p:txBody><a:bodyPr anchor="ctr"/><a:lstStyle/><a:p><a:pPr algn="ctr"/><a:r><a:rPr sz="2000"/><a:t>Boxed</a:t></a:r></a:p></p:txBody></p:sp>
<p:cxnSp><p:nvCxnSpPr><p:cNvPr id="12" name="Arrow"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr><p:spPr><a:xfrm flipV="1"><a:off x="0" y="0"/><a:ext cx="400000" cy="200000"/></a:xfrm><a:prstGeom prst="line"><a:avLst/></a:prstGeom><a:ln w="19050"><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></a:ln></p:spPr></p:cxnSp>
</p:grpSp>
<p:sp><p:nvSpPr><p:cNvPr id="20" name="Link" descr="Go to slide two"><a:hlinkClick r:id="rId9" action="ppaction://hlinksldjump"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="100000" y="5000000"/><a:ext cx="1000000" cy="400000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="333333"/></a:solidFill></p:spPr></p:sp>
</p:spTree></p:cSld><p:transition spd="fast"><p:fade/></p:transition></p:sld>"#)
}

fn slide2() -> String {
    format!(r#"<p:sld {NS}><p:cSld><p:bg><p:bgPr><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="000000"/></a:gs><a:gs pos="100000"><a:srgbClr val="FFFFFF"/></a:gs></a:gsLst><a:lin ang="5400000"/></a:gradFill></p:bgPr></p:bg><p:spTree>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="Table"/></p:nvGraphicFramePr><p:xfrm><a:off x="1000000" y="1000000"/><a:ext cx="4000000" cy="1000000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1"/><a:tblGrid><a:gridCol w="2000000"/><a:gridCol w="2000000"/></a:tblGrid>
<a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>A1</a:t></a:r></a:p></a:txBody><a:tcPr><a:solidFill><a:srgbClr val="CCDDEE"/></a:solidFill></a:tcPr></a:tc><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>B1</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
<a:tr h="500000"><a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Wide</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc hMerge="1"><a:txBody><a:bodyPr/><a:lstStyle/><a:p/></a:txBody><a:tcPr/></a:tc></a:tr>
</a:tbl></a:graphicData></a:graphic></p:graphicFrame>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="Triangle"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm rot="5400000"><a:off x="6000000" y="1000000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm><a:prstGeom prst="triangle"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="00AA00"/></a:solidFill></p:spPr></p:sp>
</p:spTree></p:cSld><p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" nodeType="tmRoot"><p:childTnLst><p:seq><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst>
<p:par><p:cTn id="3" fill="hold"><p:stCondLst><p:cond delay="indefinite"/></p:stCondLst><p:childTnLst><p:par><p:cTn id="4" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst>
<p:par><p:cTn id="5" presetID="2" presetClass="entr" presetSubtype="8" nodeType="clickEffect"><p:stCondLst><p:cond delay="200"/></p:stCondLst><p:childTnLst><p:set><p:cBhvr><p:cTn id="6" dur="1"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:set><p:anim><p:cBhvr><p:cTn id="7" dur="700"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:anim></p:childTnLst></p:cTn></p:par>
<p:par><p:cTn id="8" presetID="8" presetClass="emph" nodeType="withEffect" repeatCount="2000"><p:stCondLst><p:cond delay="100"/></p:stCondLst><p:childTnLst><p:animRot by="21600000"><p:cBhvr><p:cTn id="9" dur="1000" fill="hold"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animRot></p:childTnLst></p:cTn></p:par>
<p:par><p:cTn id="10" presetID="0" presetClass="path" nodeType="withEffect"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:animMotion origin="layout" path="M 0 0 L 0.25 0 C 0.3 0.1 0.35 0.1 0.4 0 E" pathEditMode="relative"><p:cBhvr><p:cTn id="11" dur="1500" fill="hold"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animMotion></p:childTnLst></p:cTn></p:par>
</p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:seq></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing></p:sld>"#)
}

fn notes() -> String {
    format!(r#"<p:notes {NS}><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id="2" name="Notes"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>Say hello.</a:t></a:r></a:p><a:p><a:r><a:t>Then continue.</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#)
}

fn build() -> Vec<u8> {
    let pres = format!(r#"<p:presentation {NS}><p:sldMasterIdLst><p:sldMasterId id="1" r:id="rId1"/></p:sldMasterIdLst>
<p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId3"/><p:sldId id="258" r:id="rId4"/></p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/>
<p:extLst><p:ext><p14:sectionLst xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main"><p14:section name="Intro"><p14:sldIdLst><p14:sldId id="256"/></p14:sldIdLst></p14:section><p14:section name="Details"><p14:sldIdLst><p14:sldId id="257"/><p14:sldId id="258"/></p14:sldIdLst></p14:section></p14:sectionLst></p:ext></p:extLst></p:presentation>"#);
    let hidden = format!(r#"<p:sld {NS} show="0"><p:cSld><p:spTree/></p:cSld></p:sld>"#);
    let files: Vec<(&str, String)> = vec![
        ("_rels/.rels", rels(&[("rId1", "officeDocument", "ppt/presentation.xml")])),
        ("ppt/presentation.xml", pres),
        ("ppt/_rels/presentation.xml.rels", rels(&[("rId1", "slideMaster", "slideMasters/slideMaster1.xml"), ("rId2", "slide", "slides/slide1.xml"), ("rId3", "slide", "slides/slide2.xml"), ("rId4", "slide", "slides/slide3.xml")])),
        ("ppt/slideMasters/slideMaster1.xml", master()),
        ("ppt/slideMasters/_rels/slideMaster1.xml.rels", rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"), ("rId2", "theme", "../theme/theme1.xml")])),
        ("ppt/theme/theme1.xml", theme()),
        ("ppt/slideLayouts/slideLayout1.xml", layout()),
        ("ppt/slideLayouts/_rels/slideLayout1.xml.rels", rels(&[("rId1", "slideMaster", "../slideMasters/slideMaster1.xml")])),
        ("ppt/slides/slide1.xml", slide1()),
        ("ppt/slides/_rels/slide1.xml.rels", rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"), ("rId2", "notesSlide", "../notesSlides/notesSlide1.xml"), ("rId9", "slide", "slide2.xml")])),
        ("ppt/notesSlides/notesSlide1.xml", notes()),
        ("ppt/slides/slide2.xml", slide2()),
        ("ppt/slides/_rels/slide2.xml.rels", rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml")])),
        ("ppt/slides/slide3.xml", hidden),
        ("ppt/slides/_rels/slide3.xml.rels", rels(&[("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml")])),
    ];
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in files {
        z.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(body.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn import() -> Project {
    brainbend_import_pptx::import(Cursor::new(build()), "test").unwrap()
}

fn find<'a>(s: &'a Slide, name: &str) -> &'a Object {
    s.layers[0].objects.iter().find(|o| o.name == name).unwrap_or_else(|| panic!("no object {name}: {:?}", s.layers[0].objects.iter().map(|o| &o.name).collect::<Vec<_>>()))
}

#[test]
fn size_theme_master_and_layout_come_across() {
    let p = import();
    assert_eq!((p.slides[0].width, p.slides[0].height), (1920.0, 1080.0), "scaled to a 1920-wide slide");
    assert_eq!(p.theme.name, "Test");
    assert_eq!(p.theme.heading_font, "Montserrat");
    assert_eq!(p.theme.colors[4], Rgba(255, 0, 0, 255));
    assert_eq!(p.master.background, Some(Background::Solid(Rgba(240, 240, 240, 255))));
    assert_eq!(p.master.objects.iter().filter(|o| o.placeholder.is_none()).count(), 1, "the logo bar is a master shape");
    assert_eq!(p.layouts.len(), 1);
    assert_eq!(p.layouts[0].name, "Title and Content");
    assert_eq!(p.layouts[0].objects.iter().filter(|o| o.placeholder.is_none()).count(), 1, "the dot");
    assert_eq!(p.slides[0].layout.as_deref(), Some(p.layouts[0].id.as_str()));
    // Slides follow the layout, so the master bar and the layout dot are drawn behind them.
    assert_eq!(p.backdrop(&p.slides[0]).len(), 2);
    assert_eq!(p.resolved_background(&p.slides[0]), Some(&Background::Solid(Rgba(240, 240, 240, 255))));
}

#[test]
fn placeholders_inherit_position_and_text_style_through_layout_and_master() {
    let p = import();
    let s = &p.slides[0];
    let t = find(s, "Title 1");
    assert_eq!(t.placeholder, Some(PlaceholderKind::Title));
    let k = 1920.0 / 12192000.0;
    assert!((t.x - 1219200.0 * k).abs() < 0.6, "position from the layout, got {}", t.x);
    assert!((t.w - 9144000.0 * k).abs() < 0.6);
    // Layout says 36pt centred; master says bold, color tx2 and the major font.
    assert_eq!(t.style.size, (36.0_f32 * 12700.0 * k).round());
    assert_eq!(t.style.align, Align::Center);
    assert!(t.style.bold);
    assert_eq!(t.style.color, Rgba(0x22, 0x44, 0x66, 255));
    assert_eq!(t.style.font_family, "Montserrat");
    assert_eq!(t.style.valign, VAlign::Middle, "anchor from the master");
    assert_eq!(t.text, "Hello world");
    let b = find(s, "Content");
    assert_eq!(b.placeholder, Some(PlaceholderKind::Body));
    assert!((b.y - 1600200.0 * k).abs() < 0.6, "position falls through to the master");
}

#[test]
fn text_runs_levels_and_bullets() {
    let p = import();
    let b = find(&p.slides[0], "Content");
    let lines: Vec<&str> = b.text.lines().collect();
    assert_eq!(lines[0], "• First bold red");
    assert!(lines[1].ends_with("– Second level"), "{lines:?}");
    let rich = b.rich.as_ref().expect("a bold red run differs from the box");
    let runs = &rich[0].runs;
    let red = runs.iter().find(|r| r.text == "bold red").unwrap();
    assert_eq!(red.bold, Some(true));
    assert_eq!(red.color, Some(Rgba(0xCC, 0, 0, 255)));
    assert_eq!(runs.iter().find(|r| r.text == "First ").unwrap().bold, None);
}

#[test]
fn groups_shapes_connectors_and_links() {
    let p = import();
    let s = &p.slides[0];
    let k = 1920.0 / 12192000.0;
    // The group doubles its children's size and moves them by (1000000, 1000000).
    let inner = find(s, "Inner");
    assert!((inner.x - (1_000_000.0 + 500_000.0 * 2.0) * k).abs() < 0.6);
    assert!((inner.w - 1_000_000.0 * k).abs() < 0.6);
    assert!(inner.radius > 0.0);
    assert_eq!(inner.fill, Some(Rgba(0, 255, 0, 255)), "accent2");
    assert_eq!(inner.stroke.unwrap().color, Rgba(0, 0, 0, 255));
    assert_eq!(inner.text, "Boxed");
    assert_eq!(inner.style.align, Align::Center);
    assert!(find(s, "Inner").group.is_some() && find(s, "Inner").group == find(s, "Arrow").group, "group members stay grouped");
    assert!(matches!(find(s, "Arrow").kind, Kind::LineUp), "vertical flip draws bottom-left to top-right");
    let link = find(s, "Link");
    let t = s.triggers.iter().find(|t| matches!(&t.owner, Owner::Object(id) if *id == link.id)).expect("link trigger");
    assert!(matches!(&t.action, Action::JumpToSlide(id) if *id == p.slides[1].id));
    assert_eq!(s.fade_ms, 500);
}

#[test]
fn notes_sections_hidden_slides_and_backgrounds() {
    let p = import();
    assert_eq!(p.slides.len(), 2, "the hidden slide is skipped");
    assert_eq!(p.slides[0].notes, "Say hello.\nThen continue.");
    assert_eq!(p.scenes.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Intro", "Details"]);
    assert_eq!(p.slides[0].scene, p.scenes[0].id);
    assert_eq!(p.slides[1].scene, p.scenes[1].id);
    assert!(matches!(&p.slides[1].background, Some(Background::Gradient { angle, .. }) if (*angle - 180.0).abs() < 0.1), "top-to-bottom is 180deg in CSS terms");
    assert!(p.notes.iter().any(|n| n.contains("hidden slide")));
}

#[test]
fn tables_become_cells_and_rotation_is_kept() {
    let p = import();
    let s = &p.slides[1];
    // This table has a spanning cell; it stays an editable grid with a merged cell.
    assert!(find(s, "Table r1c1").group.is_some() && find(s, "Table r1c1").table == Some((0, 0)));
    let a1 = find(s, "Table r1c1");
    assert_eq!(a1.fill, Some(Rgba(0xCC, 0xDD, 0xEE, 255)));
    assert_eq!(a1.text, "A1");
    let wide = find(s, "Table r2c1");
    assert!((wide.w - 4_000_000.0 * 1920.0 / 12192000.0).abs() < 0.6, "a spanning cell covers both columns");
    assert_eq!(wide.table_span, Some((1, 2)));
    assert!(!s.layers[0].objects.iter().any(|o| o.name == "Table r2c2"), "merged cells vanish");
    let tri = find(s, "Triangle");
    assert_eq!(tri.rotation, 90.0);
    assert!(matches!(tri.kind, Kind::Path(_)));
}

#[test]
fn imported_project_publishes_and_round_trips() {
    let p = import();
    let bytes = p.to_package().unwrap();
    let back = Project::from_bytes(&bytes).unwrap();
    assert_eq!(back.slides.len(), 2);
    let files = brainbend_publish_html::build(&back, brainbend_publish_html::Lms::None).unwrap();
    assert!(files.iter().any(|(n, _)| n == "index.html"));
    assert!(back.check().iter().all(|i| i.severity != brainbend_doc::check::Severity::Error), "{:?}", back.check().iter().map(|i| &i.message).collect::<Vec<_>>());
}

#[test]
fn rejects_files_that_are_not_presentations() {
    assert!(brainbend_import_pptx::import(Cursor::new(b"hello".to_vec()), "x").is_err());
}

#[test]
fn entrance_animations_become_timed_effects() {
    let p = import();
    let tri = find(&p.slides[1], "Triangle");
    let e = tri.entrance.expect("fly-in");
    assert_eq!(e.kind, EffectKind::SlideLeft, "preset subtype 8 flies in from the left");
    assert_eq!((e.delay_ms, e.duration_ms), (200, 700));
    assert_eq!(tri.click_steps.entrance, 1, "the fly-in waits for the first click");
    assert_eq!((tri.click_steps.emphasis, tri.click_steps.motion), (1, 1), "the spin and path start with it");
    // The same shape also spins (emphasis, twice) and follows a motion path.
    let em = tri.emphasis.expect("spin");
    assert_eq!((em.kind, em.repeat, em.duration_ms), (EmphasisKind::Spin, 2, 1000));
    let m = tri.motion.as_ref().expect("motion path");
    assert_eq!(m.points.len(), 3);
    assert!((m.points[1].0 - 0.25 * 1920.0).abs() < 1.0 && m.points[1].1.abs() < 0.01, "{:?}", m.points);
    assert!((m.points[2].0 - 0.4 * 1920.0).abs() < 1.0 && m.smooth);
    assert_eq!(m.duration_ms, 1500);
    assert!(!p.notes.iter().any(|n| n.contains("Emphasis and motion-path")), "no longer reported as unsupported");
}
