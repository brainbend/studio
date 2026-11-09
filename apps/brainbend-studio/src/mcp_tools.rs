//! The tools the MCP controller offers an AI tool, and how each one reads or changes the open project.
//!
//! Every tool runs on the app's own thread, one at a time, through the same steps the editor uses (an undo snapshot first,
//! then the change), so what an AI tool does can be undone and shows up on screen as it happens. Slides are numbered from
//! 1, as in the slide list. Positions and sizes are in slide pixels from the top left.
use super::*;
use brainbend_mcp::ToolDef;
use serde_json::{json, Value};

/// What a tool may do, which the author allows or not in the AI tools window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Access {
    /// Looks at the project and changes nothing.
    Read,
    /// Changes the open project (undoable).
    Edit,
    /// Opens, saves or publishes files on the computer. No tool uses this yet; the file tools come next.
    #[allow(dead_code)]
    Files,
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn slide_prop() -> Value { json!({ "type": "integer", "minimum": 1, "description": "The slide's number, counting from 1 as in the slide list." }) }
fn object_prop() -> Value { json!({ "type": "string", "description": "An object's id from get_slide (or its name, when that is unique on the slide)." }) }
pub(super) fn color_prop(what: &str) -> Value { json!({ "type": "string", "description": format!("{what}, as a hex color such as #1F6FEB or 1F6FEB.") }) }
fn number_prop(what: &str) -> Value { json!({ "type": "number", "description": what }) }

/// Every tool, with what it may do.
fn table() -> Vec<(Access, ToolDef)> {
    let pos = |extra: Value| {
        let mut p = json!({
            "x": number_prop("Distance of the left edge from the slide's left, in slide pixels."),
            "y": number_prop("Distance of the top edge from the slide's top, in slide pixels."),
            "width": number_prop("Width in slide pixels."),
            "height": number_prop("Height in slide pixels."),
        });
        if let (Some(a), Some(b)) = (p.as_object_mut(), extra.as_object()) { for (k, v) in b { a.insert(k.clone(), v.clone()); } }
        p
    };
    let text_look = json!({
        "size": number_prop("Font size in slide pixels."),
        "bold": { "type": "boolean" },
        "italic": { "type": "boolean" },
        "color": color_prop("The text color"),
        "align": { "type": "string", "enum": ["left", "center", "right", "justify"] },
        "alt": { "type": "string", "description": "A description for screen readers. Needed for a picture or a shape or button with no text (say what it is or does). Leave it out for anything with text on it: screen readers read the text, and alt text would replace it." },
        "decorative": { "type": "boolean", "description": "True for a shape or picture that only decorates, so screen readers skip it." },
    });
    let tool = |access, name, description, schema, read_only| (access, ToolDef { name, description, schema, read_only });
    vec![
        tool(Access::Read, "get_project", "Facts about the open project: its title (`title`, and `title_is_missing` when it still has the default), kind, number of slides, whether it has unsaved changes and where it is saved, and the player settings now (Previous / Next buttons, progress bar, slide menu, sidebar, theme, controls). Start here.", schema(json!({}), &[]), true),
        tool(Access::Read, "list_slides", "Lists the slides in order: number, id, name, kind (normal, question or results), how many objects, and how long its timeline is.", schema(json!({}), &[]), true),
        tool(Access::Read, "get_slide", "Describes one slide: its size, notes, timeline length and every object on every layer with its id, type, position, size, text, fill, whether it starts hidden and when it shows on the timeline.", schema(json!({ "slide": slide_prop() }), &["slide"]), true),
        tool(Access::Edit, "add_slide", "Adds an empty slide and returns its number. It goes after the slide numbered `after` (default: at the end).", schema(json!({ "after": slide_prop(), "name": { "type": "string", "description": "The slide's name." }, "layout": { "type": "string", "description": "A layout's name or id from get_design. The slide gets its placeholders, ready to fill. Without it, the slide follows the layout of the slide before it." } }), &[]), false),
        tool(Access::Edit, "set_player", "Sets the project's title and how the published player behaves: `title` (the project's name, shown in the player when show_title is true), the Previous / Next buttons (show_nav), the progress bar (show_progress), the slide menu (show_menu, and menu_free to let learners skip from it), the sidebar (off, left or right), player_theme (dark or light), accent (a hex color), the notes and glossary tabs, search, and the playback controls (an object of play_pause, seekbar, replay, volume, captions, speed, fullscreen). Give only what you want to change. A setting the author has already chosen is not changed until you have asked them and they agree (then pass approved: true); settings they have not touched can be set freely.",
            schema(json!({
                "title": { "type": "string", "description": "The project's title." },
                "show_title": { "type": "boolean", "description": "Show the title in a bar along the top of the player." },
                "show_nav": { "type": "boolean", "description": "The player's Previous / Next buttons." },
                "show_progress": { "type": "boolean", "description": "The progress bar along the top, filling as slides are visited." },
                "show_menu": { "type": "boolean", "description": "A slide menu (table of contents) in the player." },
                "menu_free": { "type": "boolean", "description": "Let learners jump to any slide from the menu, past unanswered questions." },
                "sidebar": { "type": "string", "enum": ["off", "left", "right"], "description": "A sidebar listing the slides, instead of a Menu button." },
                "player_theme": { "type": "string", "enum": ["dark", "light"] },
                "accent": color_prop("The player's accent color, for buttons and the current slide"),
                "notes_tab": { "type": "boolean" }, "glossary_tab": { "type": "boolean" }, "search": { "type": "boolean" },
                "controls": { "type": "object", "description": "Which playback controls to show: any of play_pause, seekbar, replay, volume, captions, speed, fullscreen, each true or false." },
                "approved": { "type": "boolean", "description": "Only after the author has agreed in chat to changing a setting they had already chosen." },
            }), &[]), false),
        tool(Access::Read, "get_design", "The lesson's look: how big a slide is, the theme (its ten named colors and two fonts), the brand palette, the master, every layout with its placeholders, and which layout each slide uses. Call this before building slides, and use only these colors and fonts.", schema(json!({}), &[]), true),
        tool(Access::Edit, "set_design", "Sets the lesson's look: theme colors (an object of color names to hex colors; the names are Text, Background, Dark accent, Light accent, Accent 1 to Accent 6), heading_font and body_font, the brand palette (a list of hex colors the author sees first in every color menu, up to 24), the master's background color, and the master's decorative shapes (shown behind every layout that shows the master). Give only what you want to change. Slides and layouts that already exist keep their colors, so do this before building them. If the author already has a theme, palette or master, this is refused until you have asked them and they agree.",
            schema(json!({
                "theme_name": { "type": "string", "description": "A name for the theme, such as the brand's." },
                "colors": { "type": "object", "description": "Theme colors by name, as hex colors: {\"Text\": \"#222222\", \"Background\": \"#FFFFFF\", \"Dark accent\": \"#1F3A5F\", \"Light accent\": \"#EEF2F7\", \"Accent 1\": \"#03539E\"}." },
                "heading_font": { "type": "string", "description": "A font family for headings, such as Montserrat." },
                "body_font": { "type": "string", "description": "A font family for body text, such as Open Sans." },
                "palette": { "type": "array", "items": { "type": "string" }, "description": "The brand colors, as hex colors, in the order the author should see them." },
                "master_background": color_prop("The background behind every slide that has no background of its own"),
                "master_shapes": { "type": "array", "items": { "type": "object" }, "description": "Decorative shapes drawn on every slide, replacing the master's current ones: each has shape (rectangle, rounded_rectangle or ellipse), x, y, width, height, fill and optionally name." },
                "approved": { "type": "boolean", "description": "Only after the author has agreed in chat to this change to the design they already have. Not needed on a lesson with no design of its own." },
            }), &[]), false),
        tool(Access::Edit, "create_layout", "Makes a layout, a page design that slides can follow: placeholders for titles and text (the same as the layout builder makes) with optional background and decorative shapes. On a lesson with a design of its own, ask the author first, and never replace a layout of theirs. Placeholders take the theme's fonts and colors; give size, color, align or bold to change one. Positions are in slide pixels and must fit on the slide (get_design says how big it is). `kind` is \"Heading 2\" for a slide title, \"Heading 3\" for a subtitle, \"Heading 4\", or \"Normal Text\" for body text.",
            schema(json!({
                "name": { "type": "string", "description": "The layout's name, such as Title and Content." },
                "placeholders": { "type": "array", "items": { "type": "object" }, "description": "Each has kind, x, y, width, height and optionally name, size, color, align, bold." },
                "shapes": { "type": "array", "items": { "type": "object" }, "description": "Decorative shapes behind the placeholders: shape, x, y, width, height, fill and optionally name." },
                "background": color_prop("A background color for slides on this layout"),
                "show_master": { "type": "boolean", "description": "Whether the master's shapes show behind this layout (default true)." },
                "approved": { "type": "boolean", "description": "Only after the author has agreed in chat to this change to the design they already have. Not needed on a lesson with no design of its own." },
            }), &["name", "placeholders"]), false),
        tool(Access::Edit, "delete_layout", "Deletes a layout, by name or id. Slides that used it keep their content and stop following a layout. Ask the author first on a lesson with a design of its own.", schema(json!({ "layout": { "type": "string", "description": "The layout's name or id." }, "approved": { "type": "boolean", "description": "Only after the author has agreed in chat to this change to the design they already have. Not needed on a lesson with no design of its own." }, }), &["layout"]), false),
        tool(Access::Edit, "apply_layout", "Puts a slide on a layout: its placeholders take the layout's places and look, keeping their text, missing placeholders are added, and empty ones the layout does not have are removed.", schema(json!({ "slide": slide_prop(), "layout": { "type": "string", "description": "The layout's name or id." } }), &["slide", "layout"]), false),
        tool(Access::Read, "find_images", "Searches for royalty-free pictures (Openverse: Wikimedia Commons, museums and other openly licensed collections) and returns candidates with a url, title, creator, license and size. By default only pictures that need no credit (CC0 and public domain) are returned; set allow_credit_required for more choice, and then pass each chosen picture's credit to add_image. Results are matched on words only, so check the title before using one. Use this for real photographs; for a diagram or chart, draw it yourself and add the file.",
            schema(json!({
                "query": { "type": "string", "description": "What the picture should show, in a few plain words, such as \"forklift warehouse\"." },
                "count": { "type": "integer", "minimum": 1, "maximum": 12, "description": "How many to return (default 6)." },
                "shape": { "type": "string", "enum": ["wide", "tall", "square"], "description": "Only pictures of this shape." },
                "allow_credit_required": { "type": "boolean", "description": "Also include pictures that need a credit line (CC BY). Default false." },
            }), &["query"]), true),
        tool(Access::Edit, "add_image", "Adds a picture to a slide, from a web address (`url`, such as one from find_images) or from a PNG or JPEG file on this computer (`path`, an absolute path: a picture you saved, drew or generated). Studio does not generate pictures itself; if you can make one, save it as PNG or JPEG and pass its path. It is fitted inside the box you give (width and height, from x and y), keeping its proportions, and centred on the slide if you give no place. Always give `alt`, a short description of what it shows. Pass `credit` for a picture that needs one, and it is added to the slide's notes. Prefer a picture over an empty box or a placeholder; do not use pictures you have no right to use.",
            schema(json!({
                "slide": slide_prop(),
                "url": { "type": "string", "description": "A web address (https) of a PNG or JPEG picture." },
                "path": { "type": "string", "description": "The full path of a PNG or JPEG file on this computer." },
                "alt": { "type": "string", "description": "What the picture shows, for people who cannot see it." },
                "decorative": { "type": "boolean", "description": "True for a picture that only decorates, so no alt is needed." },
                "credit": { "type": "string", "description": "The credit line the picture's license asks for." },
                "x": { "type": "number" }, "y": { "type": "number" },
                "width": { "type": "number", "description": "The widest the picture may be. With a height too, it fits inside that box." },
                "height": { "type": "number", "description": "The tallest the picture may be." },
                "name": { "type": "string", "description": "A name for the object." },
                "send_to_back": { "type": "boolean", "description": "Put it behind the slide's other objects, such as for a background picture." },
            }), &["slide"]), false),
        tool(Access::Read, "find_icons", "Searches the icon library by words in the icons' names (house, arrow-right, check, phone, warning, ...) and returns names to use with add_icon. Use an icon instead of a symbol or emoji character in text, which the fonts often cannot draw.",
            schema(json!({ "query": { "type": "string", "description": "A word or two, such as house or arrow right." }, "count": { "type": "integer", "minimum": 1, "maximum": 40, "description": "How many names to return (default 12)." } }), &["query"]), true),
        tool(Access::Edit, "add_icon", "Adds an icon from the library to a slide as a vector shape that scales without blurring and takes a fill color. `icon` is a name from find_icons. It is `size` pixels on its longest side (default 120) and centred on the slide unless x and y are given. Its lines are in the lesson's text color unless you give `color` (use a theme or palette color), and `fill` colors the inside of the icon, such as the body of a house. Its alt text is the icon's name unless you give `alt` or mark it decorative. For an icon button, put the icon next to or on a shape and give the shape the click trigger.",
            schema(json!({
                "slide": slide_prop(),
                "icon": { "type": "string", "description": "The icon's name, such as house or arrow-right." },
                "size": { "type": "number", "description": "Pixels on the icon's longest side (default 120)." },
                "x": { "type": "number" }, "y": { "type": "number" },
                "color": color_prop("The color of the icon's lines (its outline color)"),
                "fill": color_prop("The color of the inside of the icon, behind its lines (leave out for none)"),
                "alt": { "type": "string", "description": "What the icon means for people who cannot see it, such as Back to the home slide." },
                "decorative": { "type": "boolean", "description": "True when the icon only decorates." },
                "name": { "type": "string", "description": "A name for the object." },
                "send_to_back": { "type": "boolean" },
            }), &["slide", "icon"]), false),
        tool(Access::Edit, "add_button", "Adds a button as one object: the words inside a rounded shape, with a click trigger that goes to a slide, the next slide or the previous one. Use this for every button instead of building one from a shape and a text box. It is in the theme's accent color with text that can be read on it, centred near the bottom unless you give a place. Screen readers read the words, so give no alt text. `icon` (a name from find_icons) puts an icon beside the words.",
            schema(json!({
                "slide": slide_prop(),
                "text": { "type": "string", "description": "The words on the button." },
                "action": { "type": "string", "enum": ["go_to_slide", "next", "previous"], "description": "What pressing it does (default go_to_slide)." },
                "target": { "type": "integer", "minimum": 1, "description": "For go_to_slide: the number of the slide to go to." },
                "icon": { "type": "string", "description": "An icon name from find_icons, shown beside the words." },
                "x": { "type": "number" }, "y": { "type": "number" },
                "width": { "type": "number", "description": "Default 360." }, "height": { "type": "number", "description": "Default 100." },
                "size": { "type": "number", "description": "Text size in slide pixels (default 40)." },
                "fill": color_prop("The button's color (default the theme's Accent 1)"),
                "name": { "type": "string", "description": "A name for the object." },
            }), &["slide", "text"]), false),
        tool(Access::Edit, "group_objects", "Ties two or more objects on one layer of a slide into a group that works as one control, such as a card made of a shape, an icon, a picture and some words. The group moves and resizes together. With an `action` (go_to_slide with `target`, next or previous) a click on any part runs it, and the first object listed holds the trigger. Screen readers read the group once, by the words on its parts, or by `alt` if you give it, and its parts are not read separately. Use this instead of laying separate clickable pieces over each other; put no click trigger on the other parts.",
            schema(json!({
                "slide": slide_prop(),
                "objects": { "type": "array", "items": { "type": "string" }, "description": "Names or ids of the objects, two or more. The first one holds the click trigger." },
                "action": { "type": "string", "enum": ["go_to_slide", "next", "previous"], "description": "What a click on the group does. Leave out for a group that only moves together." },
                "target": { "type": "integer", "minimum": 1, "description": "For go_to_slide: the number of the slide to go to." },
                "alt": { "type": "string", "description": "What the group does, for screen readers, when the words on its parts do not say it. Leave out if they do." },
                "name": { "type": "string", "description": "A name for the group." },
            }), &["slide", "objects"]), false),
        tool(Access::Read, "check_accessibility", "Runs the editor's accessibility checker on the project (or one slide) and returns every error and warning: missing alt text or labels, low contrast, text laid over a clickable object, heading order, captions and more. Call it before you tell the author you are finished, and fix everything it lists.",
            schema(json!({ "slide": slide_prop() }), &[]), true),
        tool(Access::Edit, "delete_slide", "Deletes a slide. The last slide cannot be deleted.", schema(json!({ "slide": slide_prop() }), &["slide"]), false),
        tool(Access::Edit, "rename_slide", "Gives a slide a new name.", schema(json!({ "slide": slide_prop(), "name": { "type": "string" } }), &["slide", "name"]), false),
        tool(Access::Edit, "set_slide_notes", "Sets the notes of a slide (the speaker notes shown in the player's Notes tab).", schema(json!({ "slide": slide_prop(), "notes": { "type": "string" } }), &["slide", "notes"]), false),
        tool(Access::Edit, "add_text", "Adds a text box to a slide's base layer and returns its id.",
            schema({ let mut p = pos(text_look.clone()); p["slide"] = slide_prop(); p["text"] = json!({ "type": "string" }); p["name"] = json!({ "type": "string" }); p }, &["slide", "text"]), false),
        tool(Access::Edit, "add_shape", "Adds a rectangle, rounded rectangle or ellipse to a slide's base layer, optionally with text inside, and returns its id.",
            schema({ let mut p = pos(json!({})); p["slide"] = slide_prop(); p["shape"] = json!({ "type": "string", "enum": ["rectangle", "rounded_rectangle", "ellipse"] }); p["fill"] = color_prop("The fill color"); p["text"] = json!({ "type": "string" }); p["name"] = json!({ "type": "string" }); p }, &["slide", "shape"]), false),
        tool(Access::Edit, "set_object", "Changes an object: its name, text, position, size, fill, text look, whether it starts hidden, and when it shows on the slide's timeline. Only the properties you give change.",
            schema({
                let mut p = pos(text_look);
                p["slide"] = slide_prop();
                p["object"] = object_prop();
                p["name"] = json!({ "type": "string" });
                p["text"] = json!({ "type": "string" });
                p["fill"] = color_prop("The fill color");
                p["hidden"] = json!({ "type": "boolean", "description": "Starts hidden (a trigger shows it)." });
                p["start_ms"] = json!({ "type": "integer", "minimum": 0, "description": "When it appears on the timeline, in milliseconds. Setting this puts the object on the timeline." });
                p["duration_ms"] = json!({ "type": ["integer", "null"], "minimum": 0, "description": "How long it shows, in milliseconds, or null to last until the end." });
                p["always_visible"] = json!({ "type": "boolean", "description": "true shows it for the whole timeline (clears its start and duration)." });
                p
            }, &["slide", "object"]), false),
        tool(Access::Edit, "delete_object", "Deletes an object from a slide.", schema(json!({ "slide": slide_prop(), "object": object_prop() }), &["slide", "object"]), false),
        tool(Access::Read, "describe_model", "Tells you how the project is written and what can be set. With no `type` it gives an overview and lists every type; with a type name (Object, Entrance, Emphasis, Trigger, Action, Event, TextStyle, Slide, Layer, Theme, ...) it lists that type's fields, what each takes and what each does. Use it before changing anything you are not sure of.",
            schema(json!({ "type": { "type": "string", "description": "A type name, such as Object or Entrance. Leave out for the overview." } }), &[]), true),
        tool(Access::Read, "read", "Reads any part of the project as JSON, by path (JSON Pointer, counting from 0): /slides/0, /slides/0/layers/0/objects/2, /slides/0/triggers, /theme, /variables, /player. With no path, or a part too big to show, it lists what is inside and how big each part is. Pictures, video and audio files are not included.",
            schema(json!({ "path": { "type": "string", "description": "A JSON Pointer such as /slides/0/layers/0/objects/1. Leave out to see what the project contains." } }), &[]), true),
        tool(Access::Edit, "patch", "Changes anything in the project with JSON Patch (RFC 6902) operations: add, remove, replace, move, copy, test. All or nothing: it is checked against the model first and nothing changes if any part is wrong; the error says where. `add` also sets a field (replacing it if there); `-` at the end of a list's path means append. Cannot change assets, the project's kind or format. Example: [{\"op\":\"add\",\"path\":\"/slides/0/triggers/-\",\"value\":{...}}]. Use describe_model to see what each thing takes.",
            schema(json!({ "operations": { "type": "array", "items": { "type": "object" }, "description": "JSON Patch operations, each with op, path and (for add, replace, test) value, or from (for move, copy)." } }), &["operations"]), false),
        tool(Access::Edit, "create_object", "Adds an object of any kind to a slide and returns its id and path. `object` holds any properties of the object (kind, name, x, y, w, h, text, style, fill, stroke, shadow, entrance, exit, emphasis, motion, timing, states, and so on) and what you leave out takes its default. It goes on layer 0 (the base layer) unless you give `layer`, at the end unless you give `index`. An id is made for it. Use describe_model with Object for everything it can have.",
            schema(json!({ "slide": slide_prop(), "layer": { "type": "integer", "minimum": 0, "description": "The layer, counting from 0 (0 is the base layer)." }, "index": { "type": "integer", "minimum": 0, "description": "Where in the layer's list; stacking order is the list order, the last is in front." }, "object": { "type": "object", "description": "The object's properties." } }), &["slide", "object"]), false),
        tool(Access::Edit, "update_object", "Changes any properties of an object. `set` holds the fields to change: a field you give is set, a field set to null is removed (back to its default), and a field that is itself an object, such as style, is merged into rather than replaced. A list, such as a path's points, is replaced whole. Returns the object. Use describe_model with Object for everything it can have, and with Entrance, Emphasis or MotionPath for animation.",
            schema(json!({ "slide": slide_prop(), "object": object_prop(), "set": { "type": "object", "description": "The properties to change." } }), &["slide", "object", "set"]), false),
        tool(Access::Edit, "update_slide", "Changes any properties of a slide, the same way as update_object: its name, notes, size, background, transition (transition and fade_ms), whether it moves on by itself (auto_advance), navigation rules, its triggers, and so on. A list such as triggers or layers is replaced whole; to change one item in it, use patch. Returns the slide's number. Use describe_model with Slide.",
            schema(json!({ "slide": slide_prop(), "set": { "type": "object", "description": "The properties to change." } }), &["slide", "set"]), false),
        tool(Access::Edit, "undo", "Undoes the last change made to the project, whoever made it.", schema(json!({}), &[]), false),
        tool(Access::Edit, "redo", "Redoes the change that was just undone.", schema(json!({}), &[]), false),
    ]
}

/// The tools to offer.
pub fn tools() -> Vec<ToolDef> { table().into_iter().map(|(_, t)| t).collect() }

/// What a tool may do, `None` for a name that is not a tool.
pub fn access(name: &str) -> Option<Access> { table().into_iter().find(|(_, t)| t.name == name).map(|(a, _)| a) }

/// What the AI tool is told about the controller when it connects.
pub const INSTRUCTIONS: &str = "This controls BrainBend Studio, an app for building interactive lessons, while the author watches. \
Slides are numbered from 1; positions and sizes are in slide pixels from the top left (get_design says how big a slide is). \
Call get_project and list_slides first, and get_slide to see what is on a slide and the ids to change. Every change can be undone with undo. \
Work in small steps and say what you did, since the author sees the slides change as you work.\n\n\
Design before content, so the author can keep editing in the same style. Call get_design first. If the lesson has no layouts of its own (only Blank) or still has the default theme, set up its look before you build slides: \
use the author's brand colors and fonts if they gave them, otherwise choose one coherent palette and one or two fonts that suit the subject. \
Call set_design (theme colors, fonts, and the brand palette, which the author sees first in every color menu), then create_layout for each kind of page you need (for example a title slide, title and content, a section header, two columns, a closing slide), with placeholders for titles and text. \
Add every slide with add_slide and a layout, fill its placeholders with set_object, and delete a placeholder you do not use. \
Keep titles and body text in placeholders rather than loose text boxes, and use only the theme and palette colors and the theme fonts for everything else (get_design lists them). \
If the lesson already has a theme, palette, master or layouts that the author made, use them: build slides with add_slide on their layouts and their colors. The design tools refuse to change them until you have asked the author, in chat, what you would add or change and why, and they have said yes; then repeat the call with approved set to true. Never replace a layout of theirs; make a new one with another name. Everything is undoable with undo.\n\n\
Give the lesson a title and set up its player to suit it. If get_project says title_is_missing, choose a clear title from what the author asked for, call set_player with title and show_title true, and say what you chose. \
Then set the player to fit how the lesson is used. A lesson meant to be followed in order (training, a course, a tour) keeps the Previous / Next buttons and the progress bar. \
A lesson that is not in order (a hub that links to topics, a menu of choices, a presentation the author moves through with their own buttons, a kiosk or quiz bank) should have show_nav false and show_progress false, because those mean nothing there. \
If you turn the buttons off you must give learners another way to move: build your own navigation on the slides (buttons with a go-to-slide trigger, such as Home, Back and topic buttons) or turn on the slide menu; set_player warns when there is none. \
Turn on fullscreen for a presentation and play_pause and seekbar when slides have timelines, video or audio. Settings the author already chose are not changed until you ask them in chat and they agree; then repeat the call with approved set to true. Undo reverses it.\n\n\
Use real pictures, not empty boxes or placeholder text, where a slide would be better with one. For a photograph, call find_images (royalty-free, no credit needed by default) and add_image with a result's url; if a result needs a credit, pass it as credit. For a diagram, chart or illustration, draw it yourself and add_image with the file's path (PNG or JPEG); if you can generate an image, save it and add it the same way. Never use a picture you do not have the right to use.\n\n\
Make everything you build accessible yourself, so the author does not have to find and fix it. Make every button with add_button, which is one object (the words inside the shape, one click trigger). For a card or any control made of several parts (a shape with an icon, a picture and words), build the parts and tie them together with group_objects and one click action, rather than laying separate pieces over each other; never lay a text box over a clickable shape without grouping them. Give every picture, and every icon or shape with no words that stands alone, alt text saying what it shows or does, and mark pure decoration decorative; give no alt text to a button or text that has words on it, since screen readers read the words and alt text would replace them. Keep titles in the layout's heading placeholders, use colors that read well against each other, and give video and audio captions or a description. Before you tell the author you are finished, call check_accessibility and fix every error and warning it lists. \
In text and on buttons use plain words, not symbols or emoji (such as a house or arrow character): the lesson's fonts cannot draw many of them and they show as an empty square. For an icon (home, back, next, check, warning, phone) call find_icons and add_icon, which place a vector icon in the lesson's colors; give an icon that stands alone alt text saying what it does.";

/// Everything worth telling the AI about an object it just made or changed.
pub(super) fn object_warning(o: &Object) -> Option<String> {
    let mut w: Vec<String> = vec![];
    if let Some(s) = symbol_warning(&o.text) { w.push(s); }
    if !o.alt.trim().is_empty() && !o.text.trim().is_empty() {
        w.push("This object has text, which screen readers read as it is. The alt text replaces that text for screen readers, so leave it out unless it must say something different.".into());
    }
    if w.is_empty() { None } else { Some(w.join(" ")) }
}

/// A warning when `text` has symbols the fonts Studio ships with usually lack, which would show as an empty square.
pub(super) fn symbol_warning(text: &str) -> Option<String> {
    let risky: Vec<char> = text.chars().filter(|&c| matches!(c as u32, 0x2300..=0x23FF | 0x2460..=0x24FF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0x1F000..=0x1FAFF | 0xFE00..=0xFE0F)).collect();
    if risky.is_empty() { return None; }
    let mut shown: Vec<String> = vec![];
    for c in risky { let s = c.to_string(); if !shown.contains(&s) { shown.push(s); } }
    Some(format!("The text has symbols ({}) that the lesson's fonts probably cannot draw, so they may show as an empty square. Use plain words instead, or an icon from find_icons and add_icon.", shown.join(" ")))
}

// ---- reading what a tool was given ----

fn bad(key: &str, want: &str) -> String { format!("`{key}` must be {want}.") }

pub(super) fn text_arg(a: &Value, key: &str) -> Result<Option<String>, String> {
    match a.get(key) { None | Some(Value::Null) => Ok(None), Some(Value::String(s)) => Ok(Some(s.clone())), Some(_) => Err(bad(key, "text")) }
}

pub(super) fn num_arg(a: &Value, key: &str, min: f64, max: f64) -> Result<Option<f32>, String> {
    match a.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_f64() {
            Some(n) if n.is_finite() && n >= min && n <= max => Ok(Some(n as f32)),
            Some(_) => Err(format!("`{key}` must be between {min} and {max}.")),
            None => Err(bad(key, "a number")),
        },
    }
}

pub(super) fn bool_arg(a: &Value, key: &str) -> Result<Option<bool>, String> {
    match a.get(key) { None | Some(Value::Null) => Ok(None), Some(Value::Bool(b)) => Ok(Some(*b)), Some(_) => Err(bad(key, "true or false")) }
}

pub(super) fn color_arg(a: &Value, key: &str) -> Result<Option<Rgba>, String> {
    match text_arg(a, key)? {
        None => Ok(None),
        Some(t) => Rgba::parse(&t).map(Some).ok_or_else(|| format!("`{key}` is not a color I can read: {t:?}. Use a hex color such as #1F6FEB.")),
    }
}

fn align_arg(a: &Value) -> Result<Option<Align>, String> {
    match text_arg(a, "align")?.as_deref() {
        None => Ok(None),
        Some("left") => Ok(Some(Align::Left)),
        Some("center") => Ok(Some(Align::Center)),
        Some("right") => Ok(Some(Align::Right)),
        Some("justify") => Ok(Some(Align::Justify)),
        Some(other) => Err(format!("`align` must be left, center, right or justify, not {other:?}.")),
    }
}

/// A slide number (from 1) as a position in the project's list.
pub(super) fn slide_arg(a: &Value, key: &str, count: usize) -> Result<usize, String> {
    let n = a.get(key).and_then(Value::as_u64).ok_or_else(|| format!("`{key}` must be a slide number, counting from 1."))? as usize;
    if n == 0 || n > count { return Err(format!("There is no slide {n}; the project has {count} slide{}.", if count == 1 { "" } else { "s" })); }
    Ok(n - 1)
}

/// The place of an object on a slide, by id or by a name that only one object has.
pub(super) fn find_object(slide: &Slide, which: &str) -> Result<(usize, usize), String> {
    let all: Vec<(usize, usize, &Object)> = slide.layers.iter().enumerate().flat_map(|(li, l)| l.objects.iter().enumerate().map(move |(oi, o)| (li, oi, o))).collect();
    if let Some((li, oi, _)) = all.iter().find(|(_, _, o)| o.id == which) { return Ok((*li, *oi)); }
    let named: Vec<_> = all.iter().filter(|(_, _, o)| o.name == which).collect();
    match named.len() {
        1 => Ok((named[0].0, named[0].1)),
        0 => Err(format!("There is no object {which:?} on this slide. get_slide lists the objects and their ids.")),
        n => Err(format!("{n} objects on this slide are called {which:?}; use an id from get_slide.")),
    }
}

fn kind_label(o: &Object) -> &'static str {
    match &o.kind {
        Kind::Rect => "rectangle", Kind::Ellipse => "ellipse", Kind::Text => "text", Kind::Image => "picture", Kind::Video => "video", Kind::Audio => "audio",
        Kind::Hotspot => "hotspot", Kind::Path(_) => "vector shape", Kind::Line | Kind::LineUp => "line", Kind::Slider(_) => "slider", Kind::Web(_) => "web object",
        Kind::Pano(_) => "360 image", Kind::Input(_) => "input", _ => "other",
    }
}

pub(super) fn describe_object(o: &Object) -> Value {
    let mut v = json!({
        "id": o.id, "name": o.name, "type": kind_label(o),
        "x": o.x, "y": o.y, "width": o.w, "height": o.h,
        "hidden": o.hidden,
    });
    if !o.text.is_empty() { v["text"] = json!(o.text); }
    if let Some(k) = o.placeholder { v["placeholder"] = json!(k.label()); }
    if !o.alt.is_empty() { v["alt"] = json!(o.alt); }
    if o.decorative { v["decorative"] = json!(true); }
    if matches!(o.kind, Kind::Text) { v["font"] = json!(o.style.font_family); v["size"] = json!(o.style.size); v["text_color"] = json!(o.style.color.hex()); }
    if let Some(f) = o.fill { v["fill"] = json!(f.hex()); }
    if let Some(t) = o.timing { v["start_ms"] = json!(t.start_ms); v["duration_ms"] = json!(t.dur_ms); }
    v
}

fn slide_kind(s: &Slide) -> &'static str { match s.kind { SlideKind::Normal => "normal", SlideKind::Question(_) => "question", SlideKind::Results => "results" } }

fn timeline_ms(s: &Slide) -> u32 { s.layers.first().and_then(|l| l.duration_ms).unwrap_or(0) }

pub(super) fn done(v: Value) -> Result<String, String> { serde_json::to_string_pretty(&v).map_err(|e| e.to_string()) }

impl App {
    /// Runs a tool for an AI tool and says what happened, or why it could not. The caller has already checked that the
    /// author allows this kind of tool.
    pub(super) fn mcp_run(&mut self, name: &str, a: &Value) -> Result<String, String> {
        let Some(p) = self.project.as_ref() else { return Err("No project is open. Ask the author to open or create one in BrainBend Studio.".into()) };
        if p.is_web() { return Err("This is a Web Lesson, and the controller works on Media Lessons (projects of slides) for now.".into()); }
        let editing = access(name) == Some(Access::Edit);
        if editing {
            // The author is typing into a text box right now; changing the project under their cursor would lose their words.
            if self.editing.is_some() { return Err("The author is typing in a text box. Try again in a moment.".into()); }
            self.end_state_swap();
            self.leave_master_view();
        }
        let count = self.project.as_ref().map_or(0, |p| p.slides.len());
        match name {
            "get_project" => {
                let p = self.project.as_ref().unwrap();
                done(json!({
                    "name": p.name, "kind": p.kind.label(), "slides": p.slides.len(), "pass_mark_percent": p.pass_mark,
                    "unsaved_changes": self.dirty, "saved_at": self.path.as_ref().map(|p| p.display().to_string()),
                    "current_slide": self.slide + 1,
                    "player": self.player_summary(),
                }))
            }
            "list_slides" => {
                let p = self.project.as_ref().unwrap();
                done(json!(p.slides.iter().enumerate().map(|(i, s)| json!({
                    "number": i + 1, "id": s.id, "name": s.name, "kind": slide_kind(s),
                    "objects": s.layers.iter().map(|l| l.objects.len()).sum::<usize>(), "layers": s.layers.len(), "timeline_ms": timeline_ms(s),
                })).collect::<Vec<_>>()))
            }
            "get_slide" => {
                let i = slide_arg(a, "slide", count)?;
                let s = &self.project.as_ref().unwrap().slides[i];
                done(json!({
                    "number": i + 1, "id": s.id, "name": s.name, "kind": slide_kind(s), "width": s.width, "height": s.height,
                    "notes": s.notes, "timeline_ms": timeline_ms(s),
                    "layers": s.layers.iter().enumerate().map(|(li, l)| json!({ "layer": li, "name": l.name, "duration_ms": l.duration_ms, "objects": l.objects.iter().enumerate().map(|(oi, o)| { let mut d = describe_object(o); d["path"] = json!(format!("/slides/{i}/layers/{li}/objects/{oi}")); d }).collect::<Vec<_>>() })).collect::<Vec<_>>(),
                }))
            }
            "set_player" => self.mcp_set_player(a),
            "get_design" => self.mcp_get_design(),
            "set_design" => self.mcp_set_design(a),
            "create_layout" => self.mcp_create_layout(a),
            "delete_layout" => self.mcp_delete_layout(a),
            "apply_layout" => self.mcp_apply_layout(a, count),
            "find_images" => self.mcp_find_images(a),
            "add_image" => self.mcp_add_image(a, count),
            "find_icons" => self.mcp_find_icons(a),
            "add_icon" => self.mcp_add_icon(a, count),
            "add_button" => self.mcp_add_button(a, count),
            "check_accessibility" => self.mcp_check_accessibility(a),
            "group_objects" => self.mcp_group_objects(a, count),
            "add_slide" => {
                let after = match a.get("after") { None | Some(Value::Null) => count, Some(_) => slide_arg(a, "after", count)? + 1 };
                let title = text_arg(a, "name")?;
                let layout = self.layout_arg(a)?;
                self.slide = after - 1;
                match layout { Some(l) => self.add_slide_with_layout(SlideKind::Normal, Some(l)), None => self.add_slide(SlideKind::Normal) }
                if let (Some(t), Some(p)) = (title.filter(|t| !t.trim().is_empty()), self.project.as_mut()) { p.slides[self.slide].name = t.trim().to_string(); }
                self.status = format!("AI tool: added slide {}", self.slide + 1);
                let s = &self.project.as_ref().unwrap().slides[self.slide];
                done(json!({ "number": self.slide + 1, "id": s.id, "name": s.name, "width": s.width, "height": s.height, "layout": s.layout, "objects": s.layers[0].objects.iter().map(describe_object).collect::<Vec<_>>() }))
            }
            "delete_slide" => {
                let i = slide_arg(a, "slide", count)?;
                if count <= 1 { return Err("The last slide cannot be deleted.".into()); }
                self.delete_slide(i);
                self.status = format!("AI tool: deleted slide {}", i + 1);
                done(json!({ "deleted": i + 1, "slides_left": self.project.as_ref().map_or(0, |p| p.slides.len()) }))
            }
            "rename_slide" => {
                let i = slide_arg(a, "slide", count)?;
                let new = text_arg(a, "name")?.filter(|n| !n.trim().is_empty()).ok_or("`name` must be some text.")?;
                self.snapshot();
                self.project.as_mut().unwrap().slides[i].name = new.trim().to_string();
                self.slide = i;
                self.status = format!("AI tool: renamed slide {}", i + 1);
                done(json!({ "number": i + 1, "name": new.trim() }))
            }
            "set_slide_notes" => {
                let i = slide_arg(a, "slide", count)?;
                let notes = text_arg(a, "notes")?.ok_or("`notes` must be text.")?;
                self.snapshot();
                self.project.as_mut().unwrap().slides[i].notes = notes;
                self.slide = i;
                self.status = format!("AI tool: set the notes of slide {}", i + 1);
                done(json!({ "number": i + 1 }))
            }
            "add_text" | "add_shape" => self.mcp_add_object(name, a, count),
            "set_object" => self.mcp_set_object(a, count),
            "delete_object" => {
                let i = slide_arg(a, "slide", count)?;
                let which = text_arg(a, "object")?.ok_or("`object` must be an id or a name.")?;
                let (li, oi) = find_object(&self.project.as_ref().unwrap().slides[i], &which)?;
                self.snapshot();
                let removed = self.project.as_mut().unwrap().slides[i].layers[li].objects.remove(oi);
                self.slide = i;
                self.selected = None;
                self.status = format!("AI tool: deleted {} from slide {}", removed.name, i + 1);
                done(json!({ "deleted": removed.id }))
            }
            "describe_model" => brainbend_doc::model_json::describe(text_arg(a, "type")?.as_deref()),
            "read" => self.mcp_read(a),
            "patch" => self.mcp_patch(a),
            "create_object" => self.mcp_create_object(a, count),
            "update_object" => self.mcp_update(a, count, true),
            "update_slide" => self.mcp_update(a, count, false),
            "undo" => {
                if self.undo.is_empty() { return Err("There is nothing to undo.".into()); }
                self.undo();
                self.status = "AI tool: undid the last change".into();
                done(json!({ "undone": true, "more_to_undo": !self.undo.is_empty() }))
            }
            "redo" => {
                if self.redo.is_empty() { return Err("There is nothing to redo.".into()); }
                self.redo();
                self.status = "AI tool: redid a change".into();
                done(json!({ "redone": true, "more_to_redo": !self.redo.is_empty() }))
            }
            other => Err(format!("There is no tool called {other}.")),
        }
    }

    /// Text styling and the look shared by `add_text`, `add_shape` and `set_object`.
    pub(super) fn mcp_apply_look(o: &mut Object, a: &Value) -> Result<(), String> {
        if let Some(n) = num_arg(a, "size", 1.0, 1000.0)? { o.style.size = n; }
        if let Some(t) = text_arg(a, "alt")? { o.alt = t.trim().to_string(); }
        if let Some(b) = bool_arg(a, "decorative")? { o.decorative = b; }
        if let Some(b) = bool_arg(a, "bold")? { o.style.bold = b; }
        if let Some(b) = bool_arg(a, "italic")? { o.style.italic = b; }
        if let Some(c) = color_arg(a, "color")? { o.style.color = c; }
        if let Some(al) = align_arg(a)? { o.style.align = al; }
        if let Some(c) = color_arg(a, "fill")? { o.fill = Some(c); o.gradient = None; }
        Ok(())
    }

    fn mcp_add_object(&mut self, name: &str, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let (sw, sh) = { let s = &self.project.as_ref().unwrap().slides[i]; (s.width, s.height) };
        let text = text_arg(a, "text")?;
        let label = text_arg(a, "name")?;
        let (x, y) = (num_arg(a, "x", -20_000.0, 20_000.0)?, num_arg(a, "y", -20_000.0, 20_000.0)?);
        let (w, h) = (num_arg(a, "width", 1.0, 20_000.0)?, num_arg(a, "height", 1.0, 20_000.0)?);
        let mut o = if name == "add_text" {
            let text = text.ok_or("`text` must be some text.")?;
            let mut o = blank_object(self.new_id("obj"), label.as_deref().unwrap_or("Text"), Kind::Text, 0.0, 0.0, w.unwrap_or(sw * 0.5), h.unwrap_or(110.0));
            o.text = text;
            // In the lesson's own body font and text color, so loose text matches the rest.
            if let Some(p) = self.project.as_ref() { o.style.font_family = p.theme.body_font.clone(); o.style.color = p.theme.text(); }
            // Like the editor's own text box, text is anchored at the top.
            o.style.valign = VAlign::Top;
            o
        } else {
            let (kind, label_default, radius) = match text_arg(a, "shape")?.as_deref() {
                Some("rectangle") => (Kind::Rect, "Rectangle", 0.0),
                Some("rounded_rectangle") => (Kind::Rect, "Rounded rectangle", 24.0),
                Some("ellipse") => (Kind::Ellipse, "Ellipse", 0.0),
                _ => return Err("`shape` must be rectangle, rounded_rectangle or ellipse.".into()),
            };
            let accent = self.project.as_ref().map(|p| p.theme.color(4)).unwrap_or(Rgba(3, 83, 158, 255));
            let mut o = blank_object(self.new_id("obj"), label.as_deref().unwrap_or(label_default), kind, 0.0, 0.0, w.unwrap_or(sw * 0.3), h.unwrap_or(sh * 0.3));
            o.radius = radius;
            o.fill = Some(accent);
            if let Some(t) = text { o.text = t; }
            o
        };
        Self::mcp_apply_look(&mut o, a)?;
        // Without a place given, new objects start a little in from the corner and step along so they do not hide each other.
        let existing = self.project.as_ref().unwrap().slides[i].layers[0].objects.len();
        o.x = x.unwrap_or(sw * 0.08 + (existing % 8) as f32 * 36.0).round();
        o.y = y.unwrap_or(sh * 0.08 + (existing % 8) as f32 * 36.0).round();
        let mut described = describe_object(&o);
        if let Some(w) = object_warning(&o) { described["warning"] = json!(w); }
        self.snapshot();
        let slide = &mut self.project.as_mut().unwrap().slides[i];
        slide.layers[0].objects.push(o);
        self.selected = Some((0, slide.layers[0].objects.len() - 1));
        self.slide = i;
        self.layer = 0;
        self.status = format!("AI tool: added {} to slide {}", if name == "add_text" { "a text box" } else { "a shape" }, i + 1);
        done(described)
    }

    fn mcp_set_object(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let i = slide_arg(a, "slide", count)?;
        let which = text_arg(a, "object")?.ok_or("`object` must be an id or a name.")?;
        let (li, oi) = find_object(&self.project.as_ref().unwrap().slides[i], &which)?;
        // Work on a copy so that a bad value anywhere leaves the object as it was.
        let mut o = self.project.as_ref().unwrap().slides[i].layers[li].objects[oi].clone();
        if let Some(n) = text_arg(a, "name")? { o.name = n; }
        if let Some(t) = text_arg(a, "text")? { o.text = t; o.rich = None; }
        if let Some(v) = num_arg(a, "x", -20_000.0, 20_000.0)? { o.x = v; }
        if let Some(v) = num_arg(a, "y", -20_000.0, 20_000.0)? { o.y = v; }
        if let Some(v) = num_arg(a, "width", 1.0, 20_000.0)? { o.w = v; }
        if let Some(v) = num_arg(a, "height", 1.0, 20_000.0)? { o.h = v; }
        if let Some(b) = bool_arg(a, "hidden")? { o.hidden = b; }
        Self::mcp_apply_look(&mut o, a)?;
        if bool_arg(a, "always_visible")? == Some(true) {
            o.timing = None;
        } else {
            let start = num_arg(a, "start_ms", 0.0, 86_400_000.0)?.map(|v| v as u32);
            let duration = match a.get("duration_ms") {
                None => None,
                Some(Value::Null) => Some(None),
                Some(_) => Some(num_arg(a, "duration_ms", 0.0, 86_400_000.0)?.map(|v| v as u32)),
            };
            if start.is_some() || duration.is_some() {
                let current = o.timing.unwrap_or(Timing { start_ms: 0, dur_ms: None });
                o.timing = Some(Timing { start_ms: start.unwrap_or(current.start_ms), dur_ms: duration.unwrap_or(current.dur_ms) });
            }
        }
        let mut described = describe_object(&o);
        if let Some(w) = object_warning(&o) { described["warning"] = json!(w); }
        self.snapshot();
        self.project.as_mut().unwrap().slides[i].layers[li].objects[oi] = o;
        self.slide = i;
        self.layer = li;
        self.selected = Some((li, oi));
        self.status = format!("AI tool: changed an object on slide {}", i + 1);
        done(described)
    }
}

/// The most of the project one `read` shows; anything bigger gets an outline of what is inside.
const MAX_READ: usize = 30_000;

/// Each part of `v` with its place and size, for choosing what to read.
fn outline(path: &str, v: &Value) -> String {
    let size = |c: &Value| { let n = c.to_string().len(); if n >= 1024 { format!("{:.1} KB", n as f64 / 1024.0) } else { format!("{n} bytes") } };
    let rows: Vec<String> = match v {
        Value::Object(m) => m.iter().map(|(k, c)| format!("  {path}/{k} ({})", size(c))).collect(),
        Value::Array(a) => a.iter().enumerate().take(60).map(|(i, c)| format!("  {path}/{i} ({})", size(c))).collect(),
        _ => vec![],
    };
    rows.join("\n")
}

/// Problems in a project as short sentences, to tell what a change newly broke.
fn issue_lines(p: &Project) -> std::collections::BTreeSet<String> {
    p.check().iter().map(|i| format!("slide {}: {}", i.at.slide + 1, i.message)).collect()
}

/// The slide a patch is about, from the first path that names one: `/slides/3/...` is slide 4.
fn slide_in(ops: &Value) -> Option<usize> {
    ops.as_array()?.iter().find_map(|op| op.get("path")?.as_str()?.strip_prefix("/slides/")?.split('/').next()?.parse::<usize>().ok())
}

impl App {
    /// Puts a project an edit has made in place of the open one, as one undo step, and says what it newly broke.
    fn mcp_install(&mut self, new: Project, what: &str, slide: Option<usize>, select: Option<(usize, usize, usize)>) -> Value {
        let before = self.project.as_ref().map(issue_lines).unwrap_or_default();
        self.snapshot();
        let after = issue_lines(&new);
        self.project = Some(new);
        let slides = self.project.as_ref().map_or(1, |p| p.slides.len()).max(1);
        self.slide = slide.unwrap_or(self.slide).min(slides - 1);
        match select {
            Some((si, li, oi)) => { self.slide = si.min(slides - 1); self.layer = li; self.selected = Some((li, oi)); }
            None => { self.layer = 0; self.selected = None; }
        }
        self.reset_visibility();
        self.status = format!("AI tool: {what}");
        let warnings: Vec<String> = after.difference(&before).take(12).cloned().collect();
        json!({ "ok": true, "warnings": warnings })
    }

    fn mcp_read(&mut self, a: &Value) -> Result<String, String> {
        let path = text_arg(a, "path")?.unwrap_or_default();
        let project = self.project.as_mut().ok_or("No project is open.")?;
        let value = brainbend_doc::model_json::read(project, &path)?;
        let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
        if path.is_empty() || text.len() > MAX_READ {
            let inside = outline(&path, &value);
            return if path.is_empty() {
                Ok(format!("The project, by part (read a path to see one):\n{inside}\nA slide's objects are at /slides/N/layers/L/objects/I; get_slide lists them with their paths."))
            } else if inside.is_empty() {
                Err(format!("That is {} KB of text; it is too big to show.", text.len() / 1024))
            } else {
                Err(format!("That is {} KB, too big to show at once. It contains:\n{inside}\nRead one of those.", text.len() / 1024))
            };
        }
        Ok(text)
    }

    fn mcp_patch(&mut self, a: &Value) -> Result<String, String> {
        let ops = a.get("operations").cloned().ok_or("`operations` must be a list of JSON Patch operations.")?;
        let new = brainbend_doc::model_json::patch(self.project.as_mut().ok_or("No project is open.")?, &ops)?;
        let n = ops.as_array().map_or(0, Vec::len);
        let mut v = self.mcp_install(new, &format!("made {n} change{}", if n == 1 { "" } else { "s" }), slide_in(&ops), None);
        v["applied"] = json!(n);
        done(v)
    }

    fn mcp_create_object(&mut self, a: &Value, count: usize) -> Result<String, String> {
        let si = slide_arg(a, "slide", count)?;
        let layers = self.project.as_ref().unwrap().slides[si].layers.len();
        let li = match a.get("layer") { None | Some(Value::Null) => 0, Some(v) => v.as_u64().map(|n| n as usize).filter(|n| *n < layers).ok_or_else(|| format!("`layer` must be a layer number from 0 to {}.", layers - 1))? };
        let given = a.get("object").ok_or("`object` must be an object of properties.")?;
        let have = self.project.as_ref().unwrap().slides[si].layers[li].objects.len();
        let at = match a.get("index") { None | Some(Value::Null) => have, Some(v) => v.as_u64().map(|n| n as usize).filter(|n| *n <= have).ok_or_else(|| format!("`index` must be from 0 to {have}."))? };
        let id = given.get("id").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| self.new_id("obj"));
        let mut value = brainbend_doc::model_json::new_object(&id, given)?;
        value["id"] = json!(id);
        let ops = json!([{ "op": "add", "path": format!("/slides/{si}/layers/{li}/objects/{at}"), "value": value }]);
        let new = brainbend_doc::model_json::patch(self.project.as_mut().unwrap(), &ops)?;
        let mut v = self.mcp_install(new, &format!("added an object to slide {}", si + 1), Some(si), Some((si, li, at)));
        v["id"] = json!(id);
        v["path"] = json!(format!("/slides/{si}/layers/{li}/objects/{at}"));
        done(v)
    }

    /// `update_object` (`object` true) and `update_slide`: a merge patch of the fields in `set`.
    fn mcp_update(&mut self, a: &Value, count: usize, object: bool) -> Result<String, String> {
        let si = slide_arg(a, "slide", count)?;
        let set = a.get("set").filter(|s| s.is_object()).ok_or("`set` must be an object of the properties to change.")?;
        let (pointer, id) = if object {
            let which = text_arg(a, "object")?.ok_or("`object` must be an id or a name.")?;
            let slide = &self.project.as_ref().unwrap().slides[si];
            let (li, oi) = find_object(slide, &which)?;
            (format!("/slides/{si}/layers/{li}/objects/{oi}"), slide.layers[li].objects[oi].id.clone())
        } else {
            (format!("/slides/{si}"), self.project.as_ref().unwrap().slides[si].id.clone())
        };
        if set.get("id").is_some_and(|v| v.as_str() != Some(id.as_str())) { return Err("An id cannot be changed.".into()); }
        let new = brainbend_doc::model_json::merge(self.project.as_mut().unwrap(), &pointer, set)?;
        let what = if object { format!("changed an object on slide {}", si + 1) } else { format!("changed slide {}", si + 1) };
        let select = if object { new.slides[si].layers.iter().enumerate().find_map(|(li, l)| l.objects.iter().position(|o| o.id == id).map(|oi| (si, li, oi))) } else { None };
        let described = if object { select.map(|(_, li, oi)| { let mut d = describe_object(&new.slides[si].layers[li].objects[oi]); d["path"] = json!(format!("/slides/{si}/layers/{li}/objects/{oi}")); d }) } else { None };
        let mut v = self.mcp_install(new, &what, Some(si), select);
        if let Some(d) = described { v["object"] = d; } else { v["slide"] = json!(si + 1); }
        done(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with(slides: usize) -> App {
        let mut a = App::default();
        a.new_project();
        for _ in 1..slides { a.add_slide(SlideKind::Normal); }
        a.slide = 0;
        a.undo.clear();
        a.dirty = false;
        a
    }

    fn run(a: &mut App, name: &str, args: Value) -> Result<Value, String> {
        a.mcp_run(name, &args).map(|t| serde_json::from_str(&t).expect("a tool answers in JSON"))
    }

    #[test]
    fn every_tool_has_a_schema_a_description_and_an_access_level() {
        let t = tools();
        assert_eq!(t.len(), 32);
        let mut names: Vec<_> = t.iter().map(|d| d.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), t.len(), "no name is used twice");
        for d in &t {
            assert_eq!(d.schema["type"], "object", "{}", d.name);
            assert!(d.description.len() > 20, "{} needs a description an AI tool can use", d.name);
            let a = access(d.name).expect("has an access level");
            assert_eq!(d.read_only, a == Access::Read, "{}: the read-only hint matches the access level", d.name);
            for r in d.schema["required"].as_array().unwrap() { assert!(d.schema["properties"].get(r.as_str().unwrap()).is_some(), "{} requires a property it does not describe", d.name); }
        }
        assert_eq!(access("get_slide"), Some(Access::Read));
        assert_eq!(access("add_text"), Some(Access::Edit));
        assert_eq!(access("nope"), None);
    }

    #[test]
    fn nothing_works_without_a_project_or_on_a_web_lesson() {
        let mut a = App::default();
        let e = run(&mut a, "get_project", json!({})).unwrap_err();
        assert!(e.contains("No project is open"), "{e}");
        let mut w = App::default();
        w.new_lesson_project(ProjectKind::WebLesson);
        assert!(run(&mut w, "list_slides", json!({})).unwrap_err().contains("Web Lesson"));
    }

    #[test]
    fn the_project_and_its_slides_can_be_read() {
        let mut a = app_with(3);
        a.project.as_mut().unwrap().name = "Safety".into();
        let p = run(&mut a, "get_project", json!({})).unwrap();
        assert_eq!((p["name"].as_str(), p["slides"].as_u64(), p["unsaved_changes"].as_bool(), p["current_slide"].as_u64()), (Some("Safety"), Some(3), Some(false), Some(1)));
        let l = run(&mut a, "list_slides", json!({})).unwrap();
        assert_eq!(l.as_array().unwrap().len(), 3);
        assert_eq!(l[1]["number"], 2);
        assert_eq!(l[0]["kind"], "normal");
        let s = run(&mut a, "get_slide", json!({ "slide": 2 })).unwrap();
        assert_eq!(s["number"], 2);
        assert_eq!(s["layers"][0]["name"], "Base layer");
        assert!(run(&mut a, "get_slide", json!({ "slide": 9 })).unwrap_err().contains("There is no slide 9; the project has 3 slides."));
        assert!(run(&mut a, "get_slide", json!({ "slide": 0 })).is_err());
        assert!(run(&mut a, "get_slide", json!({})).unwrap_err().contains("slide number"));
        assert!(!a.dirty && a.undo.is_empty(), "reading changes nothing");
    }

    #[test]
    fn slides_can_be_added_renamed_given_notes_and_deleted() {
        let mut a = app_with(2);
        let added = run(&mut a, "add_slide", json!({ "after": 1, "name": "  Welcome " })).unwrap();
        assert_eq!((added["number"].as_u64(), added["name"].as_str()), (Some(2), Some("Welcome")), "it goes after slide 1");
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 3);
        assert!(a.dirty, "the project has unsaved changes");
        let last = run(&mut a, "add_slide", json!({})).unwrap();
        assert_eq!(last["number"], 4, "no `after` means at the end");
        run(&mut a, "rename_slide", json!({ "slide": 4, "name": "Wrap-up" })).unwrap();
        run(&mut a, "set_slide_notes", json!({ "slide": 4, "notes": "Say thanks." })).unwrap();
        let s = run(&mut a, "get_slide", json!({ "slide": 4 })).unwrap();
        assert_eq!((s["name"].as_str(), s["notes"].as_str()), (Some("Wrap-up"), Some("Say thanks.")));
        assert!(run(&mut a, "rename_slide", json!({ "slide": 4, "name": "  " })).is_err(), "a name needs some text");
        let d = run(&mut a, "delete_slide", json!({ "slide": 4 })).unwrap();
        assert_eq!((d["deleted"].as_u64(), d["slides_left"].as_u64()), (Some(4), Some(3)));
        let mut one = app_with(1);
        assert!(run(&mut one, "delete_slide", json!({ "slide": 1 })).unwrap_err().contains("last slide"));
    }

    #[test]
    fn text_and_shapes_are_added_with_a_look_and_can_be_changed() {
        let mut a = app_with(1);
        let t = run(&mut a, "add_text", json!({ "slide": 1, "text": "Hello", "x": 100, "y": 50, "width": 600, "size": 64, "bold": true, "color": "#C0392B", "align": "center", "name": "Title" })).unwrap();
        assert_eq!((t["type"].as_str(), t["text"].as_str(), t["name"].as_str(), t["x"].as_f64(), t["width"].as_f64()), (Some("text"), Some("Hello"), Some("Title"), Some(100.0), Some(600.0)));
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.style.size, o.style.bold, o.style.color, o.style.align), (64.0, true, Rgba(192, 57, 43, 255), Align::Center));
        let r = run(&mut a, "add_shape", json!({ "slide": 1, "shape": "ellipse", "fill": "1F6FEB", "text": "OK", "x": 10, "y": 10, "width": 200, "height": 100 })).unwrap();
        assert_eq!((r["type"].as_str(), r["fill"].as_str()), (Some("ellipse"), Some("#1F6FEB")));
        let id = r["id"].as_str().unwrap().to_string();
        let c = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "text": "Changed", "x": 500, "fill": "#000", "hidden": true })).unwrap();
        assert_eq!((c["text"].as_str(), c["x"].as_f64(), c["fill"].as_str(), c["hidden"].as_bool()), (Some("Changed"), Some(500.0), Some("#000000"), Some(true)));
        let by_name = run(&mut a, "set_object", json!({ "slide": 1, "object": "Title", "text": "Renamed by name" })).unwrap();
        assert_eq!(by_name["text"], "Renamed by name", "an object can be found by its name");
        let s = run(&mut a, "get_slide", json!({ "slide": 1 })).unwrap();
        assert_eq!(s["layers"][0]["objects"].as_array().unwrap().len(), 2);
        run(&mut a, "delete_object", json!({ "slide": 1, "object": id })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1);
    }

    #[test]
    fn objects_without_a_place_do_not_pile_up() {
        let mut a = app_with(1);
        let first = run(&mut a, "add_text", json!({ "slide": 1, "text": "a" })).unwrap();
        let second = run(&mut a, "add_text", json!({ "slide": 1, "text": "b" })).unwrap();
        assert_ne!((first["x"].as_f64(), first["y"].as_f64()), (second["x"].as_f64(), second["y"].as_f64()));
    }

    #[test]
    fn timing_can_be_set_changed_and_cleared() {
        let mut a = app_with(1);
        let id = run(&mut a, "add_text", json!({ "slide": 1, "text": "t" })).unwrap()["id"].as_str().unwrap().to_string();
        let t = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "start_ms": 1000, "duration_ms": 2000 })).unwrap();
        assert_eq!((t["start_ms"].as_u64(), t["duration_ms"].as_u64()), (Some(1000), Some(2000)));
        let t = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "duration_ms": null })).unwrap();
        assert_eq!((t["start_ms"].as_u64(), t["duration_ms"].clone()), (Some(1000), Value::Null), "null lasts to the end and the start stays");
        let t = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "always_visible": true })).unwrap();
        assert!(t.get("start_ms").is_none(), "no longer on the timeline");
    }

    #[test]
    fn a_bad_value_changes_nothing_and_says_what_was_wrong() {
        let mut a = app_with(1);
        let id = run(&mut a, "add_text", json!({ "slide": 1, "text": "keep" })).unwrap()["id"].as_str().unwrap().to_string();
        a.undo.clear();
        a.dirty = false;
        let before = a.project.as_ref().unwrap().slides[0].layers[0].objects[0].clone();
        for (args, wants) in [
            (json!({ "slide": 1, "object": id, "text": "new", "color": "not a color" }), "not a color I can read"),
            (json!({ "slide": 1, "object": id, "text": "new", "width": -5 }), "between"),
            (json!({ "slide": 1, "object": id, "text": "new", "width": "wide" }), "must be a number"),
            (json!({ "slide": 1, "object": id, "text": "new", "hidden": "yes" }), "true or false"),
            (json!({ "slide": 1, "object": id, "text": "new", "align": "middle" }), "left, center, right or justify"),
            (json!({ "slide": 1, "object": "missing" }), "There is no object"),
            (json!({ "slide": 1, "object": id, "text": 5 }), "must be text"),
        ] {
            let e = run(&mut a, "set_object", args).unwrap_err();
            assert!(e.contains(wants), "{e} should mention {wants}");
        }
        let after = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((after.text.as_str(), after.w), (before.text.as_str(), before.w), "the object is as it was");
        assert!(!a.dirty && a.undo.is_empty(), "and nothing was recorded as a change");
        assert!(run(&mut a, "add_shape", json!({ "slide": 1, "shape": "star" })).unwrap_err().contains("rectangle"));
    }

    #[test]
    fn two_objects_with_the_same_name_must_be_told_apart_by_id() {
        let mut a = app_with(1);
        run(&mut a, "add_text", json!({ "slide": 1, "text": "a", "name": "Label" })).unwrap();
        run(&mut a, "add_text", json!({ "slide": 1, "text": "b", "name": "Label" })).unwrap();
        let e = run(&mut a, "set_object", json!({ "slide": 1, "object": "Label", "text": "c" })).unwrap_err();
        assert!(e.contains("2 objects") && e.contains("id"), "{e}");
    }

    #[test]
    fn what_a_tool_did_can_be_undone_and_redone() {
        let mut a = app_with(1);
        run(&mut a, "add_text", json!({ "slide": 1, "text": "x" })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1);
        let u = run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(u["undone"], true);
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 0, "the text box is gone");
        assert!(run(&mut a, "undo", json!({})).unwrap_err().contains("nothing to undo"));
        run(&mut a, "redo", json!({})).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 1, "and back");
        assert!(run(&mut a, "redo", json!({})).unwrap_err().contains("nothing to redo"));
    }

    #[test]
    fn changes_wait_while_the_author_is_typing() {
        let mut a = app_with(1);
        a.editing = Some((0, 0));
        let e = run(&mut a, "add_text", json!({ "slide": 1, "text": "x" })).unwrap_err();
        assert!(e.contains("typing"), "{e}");
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), 0);
        assert!(run(&mut a, "list_slides", json!({})).is_ok(), "reading is fine while they type");
    }

    fn text_run(a: &mut App, name: &str, args: Value) -> Result<String, String> { a.mcp_run(name, &args) }

    #[test]
    fn the_model_is_described_and_any_part_of_it_can_be_read() {
        let mut a = app_with(2);
        let overview = text_run(&mut a, "describe_model", json!({})).unwrap();
        assert!(overview.contains("JSON Pointers") && overview.contains("Entrance"), "{overview}");
        let object = text_run(&mut a, "describe_model", json!({ "type": "Object" })).unwrap();
        assert!(object.contains("entrance") && object.contains("timing"), "{object}");
        assert!(text_run(&mut a, "describe_model", json!({ "type": "Entranc" })).unwrap_err().contains("Did you mean"));
        let name = text_run(&mut a, "read", json!({ "path": "/slides/1/name" })).unwrap();
        assert_eq!(name.trim(), "\"Slide 2\"");
        let whole = text_run(&mut a, "read", json!({})).unwrap();
        assert!(whole.contains("/slides (") && whole.contains("/theme (") && !whole.contains("\"layers\""), "with no path it outlines the project: {whole}");
        assert!(text_run(&mut a, "read", json!({ "path": "/nothing" })).unwrap_err().contains("nothing at"));
        assert!(!a.dirty && a.undo.is_empty(), "reading changes nothing");
    }

    #[test]
    fn something_too_big_to_show_is_outlined_instead() {
        let mut a = app_with(1);
        let long = "x".repeat(40_000);
        run(&mut a, "set_slide_notes", json!({ "slide": 1, "notes": long })).unwrap();
        let e = text_run(&mut a, "read", json!({ "path": "/slides/0" })).unwrap_err();
        assert!(e.contains("too big") && e.contains("/slides/0/notes"), "{e}");
    }

    #[test]
    fn a_patch_changes_anything_in_one_undoable_step() {
        let mut a = app_with(2);
        let r = run(&mut a, "patch", json!({ "operations": [
            { "op": "replace", "path": "/slides/1/name", "value": "Patched" },
            { "op": "add", "path": "/slides/1/notes", "value": "A note" },
            { "op": "replace", "path": "/pass_mark", "value": 70 },
        ] })).unwrap();
        assert_eq!((r["ok"].as_bool(), r["applied"].as_u64()), (Some(true), Some(3)));
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.slides[1].name.as_str(), p.slides[1].notes.as_str(), p.pass_mark), ("Patched", "A note", 70));
        assert_eq!(a.slide, 1, "the slide it worked on is on screen");
        assert!(a.dirty);
        run(&mut a, "undo", json!({})).unwrap();
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.slides[1].name.as_str(), p.slides[1].notes.as_str(), p.pass_mark), ("Slide 2", "", 80), "one undo takes back all three");
    }

    #[test]
    fn a_patch_that_is_wrong_changes_nothing_and_leaves_no_undo_step() {
        let mut a = app_with(1);
        for (ops, wants) in [
            (json!([{ "op": "add", "path": "/slides/0/layers/0/objects/-", "value": { "id": "x", "colour": "red" } }]), "invalid"),
            (json!([{ "op": "replace", "path": "/slides/0/name", "value": 5 }]), "/slides/0/name"),
            (json!([{ "op": "remove", "path": "/slides/9" }]), "could not be applied"),
            (json!([{ "op": "remove", "path": "/assets" }]), "cannot be changed"),
            (json!("not a list"), "list"),
        ] {
            let e = run(&mut a, "patch", json!({ "operations": ops })).unwrap_err();
            assert!(e.contains(wants), "{e} should mention {wants}");
        }
        assert!(!a.dirty && a.undo.is_empty(), "no failed patch is recorded");
        assert_eq!(a.project.as_ref().unwrap().slides[0].name, "Slide 1");
    }

    #[test]
    fn objects_of_any_kind_are_created_with_everything_they_can_have() {
        let mut a = app_with(1);
        let r = run(&mut a, "create_object", json!({ "slide": 1, "object": {
            "kind": "Ellipse", "name": "Dot", "x": 50, "y": 60, "w": 120, "h": 120, "fill": [200, 30, 30, 255],
            "shadow": null,
            "entrance": { "kind": "Zoom", "delay_ms": 0, "duration_ms": 500 },
            "timing": { "start_ms": 1000, "dur_ms": 3000 },
            "style": { "size": 30, "bold": true },
        } })).unwrap();
        let id = r["id"].as_str().unwrap().to_string();
        assert_eq!(r["path"], "/slides/0/layers/0/objects/0");
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.id.as_str(), o.name.as_str(), o.x, o.w), (id.as_str(), "Dot", 50.0, 120.0));
        assert!(matches!(o.kind, Kind::Ellipse) && o.entrance.is_some() && o.timing == Some(Timing { start_ms: 1000, dur_ms: Some(3000) }));
        assert_eq!((o.style.size, o.style.bold), (30.0, true));
        assert_eq!(a.selected, Some((0, 0)), "it is selected, so the author sees it");
        let second = run(&mut a, "create_object", json!({ "slide": 1, "index": 0, "object": { "name": "Behind", "kind": "Rect" } })).unwrap();
        assert_eq!(second["path"], "/slides/0/layers/0/objects/0", "an index puts it there");
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[1].name, "Dot");
        let g = run(&mut a, "get_slide", json!({ "slide": 1 })).unwrap();
        assert_eq!(g["layers"][0]["objects"][1]["path"], "/slides/0/layers/0/objects/1", "get_slide gives each object's path");
    }

    #[test]
    fn creating_an_object_checks_what_it_is_given() {
        let mut a = app_with(1);
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": { "kind": "Teleporter" } })).unwrap_err().contains("invalid"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": { "nonsense": 1 } })).unwrap_err().contains("nonsense"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "layer": 4, "object": {} })).unwrap_err().contains("layer number"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "index": 9, "object": {} })).unwrap_err().contains("`index`"));
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": "text" })).is_err());
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects.is_empty());
        let first = run(&mut a, "create_object", json!({ "slide": 1, "object": { "id": "mine" } })).unwrap();
        assert_eq!(first["id"], "mine", "an id can be chosen");
        assert!(run(&mut a, "create_object", json!({ "slide": 1, "object": { "id": "mine" } })).unwrap_err().contains("Two objects"), "but not one that is taken");
    }

    #[test]
    fn an_object_can_be_restyled_animated_and_timed_by_name_or_id() {
        let mut a = app_with(1);
        let id = run(&mut a, "create_object", json!({ "slide": 1, "object": { "kind": "Text", "name": "Headline", "text": "Hi" } })).unwrap()["id"].as_str().unwrap().to_string();
        let r = run(&mut a, "update_object", json!({ "slide": 1, "object": "Headline", "set": {
            "fill": [10, 20, 30, 255], "style": { "size": 90, "color": [255, 255, 255, 255], "align": "Center" },
            "entrance": { "kind": "Fade", "delay_ms": 100, "duration_ms": 700 },
            "emphasis": { "kind": "Pulse", "delay_ms": 0, "duration_ms": 900, "repeat": 3 },
            "exit": { "kind": "SlideLeft", "delay_ms": 0, "duration_ms": 400 },
            "timing": { "start_ms": 500, "dur_ms": 4000 },
            "shadow": { "color": [0, 0, 0, 120], "dx": 4.0, "dy": 4.0, "blur": 8.0 },
        } })).unwrap();
        assert_eq!(r["object"]["id"], id.as_str());
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert_eq!((o.text.as_str(), o.style.size, o.style.align, o.fill), ("Hi", 90.0, Align::Center, Some(Rgba(10, 20, 30, 255))), "what was not set stays");
        assert!(o.entrance.is_some() && o.emphasis.is_some() && o.exit.is_some() && o.shadow.is_some());
        assert_eq!(o.timing, Some(Timing { start_ms: 500, dur_ms: Some(4000) }));
        run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": { "entrance": null, "timing": null } })).unwrap();
        let o = &a.project.as_ref().unwrap().slides[0].layers[0].objects[0];
        assert!(o.entrance.is_none() && o.timing.is_none(), "null takes a property away");
        assert!(run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": { "id": "other" } })).unwrap_err().contains("id cannot be changed"));
        assert!(run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": { "style": { "size": "huge" } } })).unwrap_err().contains("invalid"));
        assert!(run(&mut a, "update_object", json!({ "slide": 1, "object": id, "set": [] })).is_err());
    }

    #[test]
    fn a_slide_can_be_given_a_transition_and_triggers() {
        let mut a = app_with(2);
        let layer = run(&mut a, "patch", json!({ "operations": [{ "op": "add", "path": "/slides/1/layers/-", "value": {
            "id": "pop", "name": "Pop-up", "objects": [], "duration_ms": null, "modal": true, "hide_others": false, "pause_base": false
        } }] })).unwrap();
        assert_eq!(layer["ok"], true);
        let r = run(&mut a, "update_slide", json!({ "slide": 2, "set": { "fade_ms": 600, "transition": { "kind": "Wipe", "from": "Left", "inward": false } } })).unwrap();
        assert_eq!(r["slide"], 2);
        let s = &a.project.as_ref().unwrap().slides[1];
        assert_eq!((s.fade_ms, s.transition.kind), (600, TransitionKind::Wipe));
        let t = run(&mut a, "patch", json!({ "operations": [{ "op": "add", "path": "/slides/1/triggers/-", "value": {
            "id": "t1", "owner": "Slide", "event": "SlideStart", "action": { "ShowLayer": "pop" }, "conditions": []
        } }] })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[1].triggers.len(), 1);
        assert_eq!(t["warnings"].as_array().unwrap().len(), 0, "a trigger that shows a layer that exists is fine");
        assert!(run(&mut a, "update_slide", json!({ "slide": 2, "set": { "id": "x" } })).is_err());
    }

    #[test]
    fn a_change_that_newly_breaks_something_comes_back_with_a_warning() {
        let mut a = app_with(1);
        let r = run(&mut a, "patch", json!({ "operations": [{ "op": "add", "path": "/slides/0/triggers/-", "value": {
            "id": "t1", "owner": "Slide", "event": "SlideStart", "action": { "ShowLayer": "no-such-layer" }, "conditions": []
        } }] })).unwrap();
        let w = r["warnings"].as_array().unwrap();
        assert!(!w.is_empty(), "the checker found a trigger pointing at nothing: {r}");
        assert!(w[0].as_str().unwrap().starts_with("slide 1: "), "{w:?}");
        let again = run(&mut a, "patch", json!({ "operations": [{ "op": "replace", "path": "/slides/0/name", "value": "Renamed" }] })).unwrap();
        assert_eq!(again["warnings"].as_array().unwrap().len(), 0, "only what the change broke is reported, not what was already wrong");
    }

    #[test]
    fn the_models_enum_values_are_found_by_describing_the_type() {
        let mut a = app_with(1);
        let kinds = text_run(&mut a, "describe_model", json!({ "type": "Kind" })).unwrap();
        assert!(kinds.contains("Rect") && kinds.contains("Video"), "{kinds}");
        let trigger = text_run(&mut a, "describe_model", json!({ "type": "Trigger" })).unwrap();
        assert!(trigger.contains("event") && trigger.contains("action") && trigger.contains("conditions"), "{trigger}");
        assert!(text_run(&mut a, "describe_model", json!({ "type": "Action" })).unwrap().contains("ShowLayer"));
    }


    fn title_and_content() -> Value {
        json!({ "name": "Title and Content", "placeholders": [
            { "kind": "Heading 2", "name": "Title", "x": 120, "y": 80, "width": 1680, "height": 160 },
            { "kind": "Normal Text", "name": "Body", "x": 120, "y": 280, "width": 1680, "height": 640, "size": 44 },
        ], "shapes": [{ "shape": "rectangle", "x": 0, "y": 0, "width": 40, "height": 1080, "fill": "#E87722", "name": "Accent bar" }] })
    }

    #[test]
    fn the_design_can_be_read_and_set_and_new_content_uses_it() {
        let mut a = app_with(1);
        let d = run(&mut a, "get_design", json!({})).unwrap();
        assert_eq!(d["theme"]["colors"].as_array().unwrap().len(), 10);
        assert_eq!(d["theme"]["colors"][4]["name"], "Accent 1");
        assert_eq!(d["layouts"].as_array().unwrap().len(), 1, "a new lesson has the one blank layout");
        assert!(d["slide_size"]["width"].as_f64().unwrap() > 0.0);
        let r = run(&mut a, "set_design", json!({
            "theme_name": "Acme", "colors": { "Text": "#1A1A2E", "accent_1": "0F4C81", "Dark accent": "#16213E" },
            "heading_font": "Montserrat", "body_font": "Open Sans", "palette": ["#0F4C81", "#E87722", "#0F4C81"],
            "master_background": "#FAFAFA", "master_shapes": [{ "shape": "rectangle", "x": 0, "y": 1040, "width": 1920, "height": 40, "fill": "#0F4C81", "name": "Footer band" }],
        })).unwrap();
        assert_eq!(r["changed"].as_array().unwrap().len(), 7);
        let p = a.project.as_ref().unwrap();
        assert_eq!((p.theme.name.as_str(), p.theme.text().hex().as_str(), p.theme.color(4).hex().as_str(), p.theme.color(2).hex().as_str()), ("Acme", "#1A1A2E", "#0F4C81", "#16213E"), "colors are found by name however it is written");
        assert_eq!((p.theme.heading_font.as_str(), p.theme.body_font.as_str()), ("Montserrat", "Open Sans"));
        assert_eq!(p.palette.len(), 2, "a repeated brand color is kept once, in order");
        assert_eq!(p.master.objects.len(), 1);
        assert!(matches!(p.master.background, Some(brainbend_doc::master::Background::Solid(c)) if c.hex() == "#FAFAFA"));
        // A text box added now is in the lesson's own font and color.
        let t = run(&mut a, "add_text", json!({ "slide": 1, "text": "Hello" })).unwrap();
        assert_eq!((t["font"].as_str(), t["text_color"].as_str()), (Some("Open Sans"), Some("#1A1A2E")));
        // Undo takes the whole design back.
        let before_text = a.undo.len();
        run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(a.undo.len(), before_text - 1);
        run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(a.project.as_ref().unwrap().theme.name, "BrainBend Studio", "the theme, palette and master were part of the undo step");
        assert!(a.project.as_ref().unwrap().palette.is_empty() && a.project.as_ref().unwrap().master.objects.is_empty());
    }

    #[test]
    fn a_bad_design_is_refused_with_a_reason_and_changes_nothing() {
        let mut a = app_with(1);
        let before = a.project.as_ref().unwrap().theme.clone();
        for (args, expect) in [
            (json!({}), "Give at least one"),
            (json!({ "colors": { "Accent 9": "#112233" } }), "no color called"),
            (json!({ "colors": { "Accent 1": "blue-ish" } }), "not a color I can read"),
            (json!({ "heading_font": "Definitely Not A Font" }), "not available"),
            (json!({ "palette": ["#12"] }), "not a color I can read"),
            (json!({ "colors": { "Text": "#111111" }, "heading_font": "Nope Sans" }), "not available"),
            (json!({ "master_shapes": [{ "x": 0, "y": 0, "width": 10 }] }), "`height` is needed"),
        ] {
            let e = run(&mut a, "set_design", args).unwrap_err();
            assert!(e.contains(expect), "{e}");
        }
        assert_eq!(a.project.as_ref().unwrap().theme, before, "nothing was half done");
        assert!(a.undo.is_empty(), "and no undo step was made for it");
    }

    #[test]
    fn layouts_are_made_with_placeholders_and_slides_follow_them() {
        let mut a = app_with(1);
        run(&mut a, "set_design", json!({ "colors": { "Text": "#1A1A2E" }, "heading_font": "Montserrat" })).unwrap();
        let l = run(&mut a, "create_layout", title_and_content()).unwrap();
        assert_eq!(l["placeholders"].as_array().unwrap().len(), 2);
        let id = l["id"].as_str().unwrap().to_string();
        let p = a.project.as_ref().unwrap();
        let layout = p.layout(&id).unwrap();
        assert_eq!(layout.objects.len(), 3, "the shape, then the placeholders");
        let title = layout.objects.iter().find(|o| o.name == "Title").unwrap();
        assert_eq!((title.style.font_family.as_str(), title.style.color.hex().as_str()), ("Montserrat", "#1A1A2E"), "a placeholder takes the theme");
        assert_eq!(layout.objects.iter().find(|o| o.name == "Body").unwrap().style.size, 44.0, "and what was asked for");
        // A slide on that layout, by name, ready to fill.
        let s = run(&mut a, "add_slide", json!({ "layout": "title and content", "name": "Welcome" })).unwrap();
        assert_eq!(s["layout"], json!(id));
        let objs = s["objects"].as_array().unwrap();
        assert_eq!(objs.len(), 2, "the placeholders, not the decoration");
        assert!(objs.iter().all(|o| o["placeholder"].is_string() && o.get("text").is_none()), "{objs:?}");
        run(&mut a, "set_object", json!({ "slide": 2, "object": "Title", "text": "Welcome aboard" })).unwrap();
        let got = run(&mut a, "get_slide", json!({ "slide": 2 })).unwrap();
        let t = got["layers"][0]["objects"].as_array().unwrap().iter().find(|o| o["name"] == "Title").unwrap().clone();
        assert_eq!((t["text"].as_str(), t["placeholder"].as_str(), t["font"].as_str()), (Some("Welcome aboard"), Some("Heading 2"), Some("Montserrat")));
        // The next slide follows the same layout without being told.
        let next = run(&mut a, "add_slide", json!({})).unwrap();
        assert_eq!(next["layout"], json!(id), "a slide follows the one before it");
        let d = run(&mut a, "get_design", json!({})).unwrap();
        assert_eq!(d["slides"][1]["layout"], "Title and Content");
        assert_eq!(d["slides"][0]["layout"], Value::Null);
    }

    #[test]
    fn a_design_the_author_made_is_not_changed_without_their_say_so() {
        let mut a = app_with(1);
        // Nothing there yet: the tool sets up a look, and goes on adjusting its own work freely.
        run(&mut a, "set_design", json!({ "colors": { "Accent 1": "#0F4C81" }, "palette": ["#0F4C81"] })).unwrap();
        run(&mut a, "create_layout", title_and_content()).unwrap();
        run(&mut a, "set_design", json!({ "heading_font": "Montserrat" })).unwrap();
        run(&mut a, "create_layout", json!({ "name": "TITLE AND CONTENT", "placeholders": [{ "kind": "title", "x": 100, "y": 100, "width": 800, "height": 200 }] })).unwrap();
        // The author adds a brand color of their own.
        a.project.as_mut().unwrap().palette.push(Rgba(1, 2, 3, 255));
        let before = (a.project.as_ref().unwrap().theme.clone(), a.project.as_ref().unwrap().palette.clone(), a.project.as_ref().unwrap().layouts.len());
        let undo = a.undo.len();
        for (tool, args) in [
            ("set_design", json!({ "palette": ["#FF0000"] })),
            ("create_layout", json!({ "name": "Closing", "placeholders": [{ "kind": "title", "x": 100, "y": 100, "width": 800, "height": 200 }] })),
            ("create_layout", title_and_content()),
            ("delete_layout", json!({ "layout": "Title and Content" })),
        ] {
            let e = run(&mut a, tool, args).unwrap_err();
            assert!(e.contains("already has a design of its own") || e.contains("not yours to replace"), "{tool}: {e}");
            assert!(e.contains("approved"), "{tool}: {e}");
        }
        let now = (a.project.as_ref().unwrap().theme.clone(), a.project.as_ref().unwrap().palette.clone(), a.project.as_ref().unwrap().layouts.len());
        assert_eq!(before, now, "nothing was changed");
        assert_eq!(a.undo.len(), undo, "and no undo step was made");
        // Using what is there needs no say-so, and neither does reading it.
        run(&mut a, "get_design", json!({})).unwrap();
        run(&mut a, "add_slide", json!({ "layout": "Title and Content" })).unwrap();
        // With the author's yes, the change goes ahead as one undo step.
        run(&mut a, "create_layout", json!({ "name": "Closing", "approved": true, "placeholders": [{ "kind": "title", "x": 100, "y": 100, "width": 800, "height": 200 }] })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), before.2 + 1);
        run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), before.2);
        // A replacement of a layout by name needs the yes too, and says what is being replaced.
        let r = run(&mut a, "create_layout", json!({ "name": "title and content", "approved": true, "placeholders": [{ "kind": "title", "x": 100, "y": 100, "width": 800, "height": 200 }] })).unwrap();
        assert_eq!(r["replaced_one_of_the_same_name"], true);
    }

    #[test]
    fn a_lesson_that_was_opened_with_a_design_counts_as_the_authors() {
        let mut a = app_with(1);
        run(&mut a, "create_layout", title_and_content()).unwrap();
        // A fresh start of the app forgets what the tool made, so what is in the lesson is treated as the author's.
        a.mcp.design_seen = None;
        let e = run(&mut a, "set_design", json!({ "theme_name": "Other" })).unwrap_err();
        assert!(e.contains("1 layout (Title and Content)"), "{e}");
        // A lesson with only the blank layout and the stock theme has none.
        let mut b = app_with(1);
        run(&mut b, "set_design", json!({ "theme_name": "Fresh" })).unwrap();
        assert!(INSTRUCTIONS.contains("approved"));
    }

    /// A stand-in for the picture search and the host of the pictures: answers /v1/images/ with two results and /pic.png with a picture.
    fn picture_server(side: u32) -> (String, std::sync::mpsc::Receiver<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        let png = {
            let img = image::RgbaImage::from_pixel(side, side / 2, image::Rgba([10, 120, 200, 255]));
            let mut out = std::io::Cursor::new(Vec::new());
            img.write_to(&mut out, image::ImageFormat::Png).unwrap();
            out.into_inner()
        };
        let site = base.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { break };
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let line = String::from_utf8_lossy(&buf[..n]).lines().next().unwrap_or("").to_string();
                let _ = tx.send(line.clone());
                let (kind, body): (&str, Vec<u8>) = if line.contains("/v1/images/") {
                    ("application/json", json!({ "results": [
                        { "url": format!("{site}/pic.png"), "title": "Forklift", "creator": "A. Photographer", "license": "cc0", "license_version": "1.0", "width": 800, "height": 400, "foreign_landing_url": "https://example.org/forklift" },
                        { "url": format!("{site}/pic.png"), "title": "Warehouse", "creator": "B. Person", "license": "by", "license_version": "4.0", "attribution": "\"Warehouse\" by B. Person is licensed under CC BY 4.0.", "width": 800, "height": 400, "foreign_landing_url": "https://example.org/w" },
                        { "title": "no address" },
                    ] }).to_string().into_bytes())
                } else if line.contains("/pic.png") { ("image/png", png.clone()) } else { ("text/html", b"<html>not a picture</html>".to_vec()) };
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(&body);
            }
        });
        (base, rx)
    }

    #[test]
    fn pictures_are_found_and_added_with_alt_text_and_a_credit() {
        let (base, seen) = picture_server(800);
        let mut a = app_with(1);
        a.mcp.image_search_url = Some(format!("{base}/v1/images/"));
        let r = run(&mut a, "find_images", json!({ "query": "forklift warehouse", "count": 3 })).unwrap();
        let line = seen.recv().unwrap();
        assert!(line.contains("q=forklift") && line.contains("license=cc0%2Cpdm") || line.contains("license=cc0,pdm"), "only pictures that need no credit are asked for: {line}");
        assert_eq!(r["found"], 2, "a result with no address is left out");
        assert_eq!((r["results"][0]["credit_required"].clone(), r["results"][1]["credit_required"].clone()), (json!(false), json!(true)));
        assert!(r["results"][1]["credit"].as_str().unwrap().contains("CC BY 4.0"));
        let allowed = run(&mut a, "find_images", json!({ "query": "x", "allow_credit_required": true })).unwrap();
        assert_eq!(allowed["found"], 2);
        assert!(seen.recv().unwrap().contains("by"), "credit-required pictures are asked for only on request");
        let url = r["results"][1]["url"].as_str().unwrap().to_string();
        // Added with a box: fitted inside it, proportions kept, centred in it.
        let r = run(&mut a, "add_image", json!({ "slide": 1, "url": url, "alt": "A warehouse", "credit": "\"Warehouse\" by B. Person (CC BY 4.0)", "x": 100, "y": 200, "width": 400, "height": 400 })).unwrap();
        let o = &r["added"];
        assert_eq!((o["type"].as_str(), o["width"].as_f64(), o["height"].as_f64()), (Some("picture"), Some(400.0), Some(200.0)), "{o}");
        assert_eq!((o["x"].as_f64(), o["y"].as_f64()), (Some(100.0), Some(300.0)), "a wide picture sits in the middle of a taller box");
        let p = a.project.as_ref().unwrap();
        let obj = p.slides[0].layers[0].objects.last().unwrap();
        assert_eq!(obj.alt, "A warehouse");
        let asset = p.assets.get(obj.asset.as_ref().unwrap()).unwrap();
        assert_eq!((asset.mime.as_str(), asset.name.as_str()), ("image/png", "pic.png"));
        assert!(p.slides[0].notes.contains("Image credit: \"Warehouse\" by B. Person"), "{:?}", p.slides[0].notes);
        // With no place given it is centred on the slide, and a second picture can go behind the rest.
        let r = run(&mut a, "add_image", json!({ "slide": 1, "url": format!("{base}/pic.png"), "decorative": true, "send_to_back": true })).unwrap();
        assert_eq!(r["added"]["x"].as_f64().unwrap() * 2.0 + r["added"]["width"].as_f64().unwrap(), 1920.0);
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects[0].name, "Picture");
        // One undo step takes a picture back.
        let n = a.project.as_ref().unwrap().slides[0].layers[0].objects.len();
        run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), n - 1);
    }

    #[test]
    fn a_picture_the_tool_saved_is_added_and_a_bad_one_is_refused_with_a_reason() {
        let (base, _) = picture_server(4000);
        let mut a = app_with(1);
        let dir = std::env::temp_dir().join(format!("bb-img-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("chart.png");
        image::RgbaImage::from_pixel(300, 150, image::Rgba([200, 30, 30, 255])).save(&file).unwrap();
        let r = run(&mut a, "add_image", json!({ "slide": 1, "path": file.display().to_string(), "alt": "A red chart", "width": 600 })).unwrap();
        assert_eq!((r["added"]["width"].as_f64(), r["added"]["height"].as_f64()), (Some(600.0), Some(300.0)));
        // A very large picture is made smaller before it is stored.
        let r = run(&mut a, "add_image", json!({ "slide": 1, "url": format!("{base}/pic.png"), "alt": "Big" })).unwrap();
        assert_eq!(r["picture_pixels"], json!([2400, 1200]));
        // A WebP (which some picture hosts send whatever was asked for) is converted to a picture Studio keeps.
        let webp = dir.join("photo.webp");
        image::RgbaImage::from_pixel(64, 32, image::Rgba([20, 90, 160, 255])).save(&webp).unwrap();
        let r = run(&mut a, "add_image", json!({ "slide": 1, "path": webp.display().to_string(), "alt": "A WebP" })).unwrap();
        let p = a.project.as_ref().unwrap();
        let kept = p.assets.get(p.slides[0].layers[0].objects.last().unwrap().asset.as_ref().unwrap()).unwrap();
        assert_eq!((r["picture_pixels"].clone(), kept.mime.as_str()), (json!([64, 32]), "image/jpeg"), "no see-through parts, so a JPEG");
        std::fs::write(dir.join("page.txt"), "hello").unwrap();
        let before = a.project.as_ref().unwrap().slides[0].layers[0].objects.len();
        for (args, expect) in [
            (json!({ "slide": 1, "alt": "x" }), "either `url`"),
            (json!({ "slide": 1, "url": "https://a/b.png", "path": "/x.png", "alt": "x" }), "either `url`"),
            (json!({ "slide": 1, "path": file.display().to_string() }), "`alt` is needed"),
            (json!({ "slide": 1, "path": "relative.png", "alt": "x" }), "full path"),
            (json!({ "slide": 1, "path": dir.join("missing.png").display().to_string(), "alt": "x" }), "could not be found"),
            (json!({ "slide": 1, "path": dir.join("page.txt").display().to_string(), "alt": "x" }), "not a picture Studio can read"),
            (json!({ "slide": 1, "url": "file:///etc/passwd", "alt": "x" }), "starting with https://"),
            (json!({ "slide": 1, "url": format!("{base}/page.html"), "alt": "x" }), "not a picture Studio can read"),
        ] {
            let e = run(&mut a, "add_image", args).unwrap_err();
            assert!(e.contains(expect), "{e}");
        }
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), before, "nothing was added");
        let e = run(&mut a, "find_images", json!({ "query": "  " })).unwrap_err();
        assert!(e.contains("`query`"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_title_and_player_are_set_for_an_untouched_project_and_undone_in_one_step() {
        let mut a = app_with(3);
        let p = run(&mut a, "get_project", json!({})).unwrap();
        assert_eq!((p["player"]["title_is_missing"].clone(), p["player"]["previous_next_buttons"].clone()), (json!(true), json!(true)));
        let r = run(&mut a, "set_player", json!({ "title": "Warehouse Safety", "show_title": true, "show_nav": false, "show_progress": false, "show_menu": true, "controls": { "fullscreen": true }, "sidebar": "left", "accent": "#E87722" })).unwrap();
        assert_eq!(r["changed"].as_array().unwrap().len(), 8);
        assert_eq!(r["warnings"].as_array().unwrap().len(), 0, "the slide menu is a way to move");
        let pr = a.project.as_ref().unwrap();
        assert_eq!((pr.name.as_str(), pr.show_nav, pr.show_progress, pr.show_menu, pr.player.show_title, pr.player.controls.fullscreen), ("Warehouse Safety", false, false, true, true, true));
        assert_eq!((pr.player.sidebar, pr.player.accent.hex().as_str()), (SidebarSide::Left, "#E87722"));
        assert_eq!(run(&mut a, "get_project", json!({})).unwrap()["player"]["title_is_missing"], false);
        // Its own settings can be adjusted again without asking.
        run(&mut a, "set_player", json!({ "show_progress": true })).unwrap();
        // One undo step takes the whole change back, the title too.
        let steps = a.undo.len();
        run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(a.undo.len(), steps - 1);
        run(&mut a, "undo", json!({})).unwrap();
        let pr = a.project.as_ref().unwrap();
        assert_eq!((pr.name.as_str(), pr.show_nav, pr.show_menu, pr.player.show_title), ("Untitled", true, false, false));
    }

    #[test]
    fn what_the_author_chose_is_not_changed_without_their_say_so_and_losing_navigation_is_warned_about() {
        let mut a = app_with(3);
        {
            let p = a.project.as_mut().unwrap();
            p.name = "Safety Basics".into();
            p.show_progress = false;
        }
        let undo = a.undo.len();
        let e = run(&mut a, "set_player", json!({ "title": "Something Else", "show_progress": true, "show_title": true })).unwrap_err();
        assert!(e.contains("\"Safety Basics\"") && e.contains("the progress bar is off") && e.contains("approved"), "{e}");
        assert!(!e.contains("showing the title"), "only what the author chose is listed: {e}");
        let pr = a.project.as_ref().unwrap();
        assert_eq!((pr.name.as_str(), pr.show_progress, pr.player.show_title, a.undo.len()), ("Safety Basics", false, false, undo), "nothing changed, and no undo step");
        // Settings they did not touch are free, and what is already set is not a change.
        let r = run(&mut a, "set_player", json!({ "show_title": true, "show_progress": false })).unwrap();
        assert_eq!(r["changed"], json!(["show_title"]));
        // With their yes it goes ahead.
        run(&mut a, "set_player", json!({ "title": "Something Else", "approved": true })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().name, "Something Else");
        // Without Previous / Next, with several slides and no way to move, the tool says so.
        let r = run(&mut a, "set_player", json!({ "show_nav": false })).unwrap();
        assert!(r["warnings"][0].as_str().unwrap().contains("cannot move"), "{r}");
        run(&mut a, "set_player", json!({ "show_menu": true })).unwrap();
        // A single slide needs no navigation.
        let mut b = app_with(1);
        assert_eq!(run(&mut b, "set_player", json!({ "show_nav": false })).unwrap()["warnings"].as_array().unwrap().len(), 0);
        for (args, expect) in [(json!({}), "Give at least one"), (json!({ "approved": true }), "Give at least one"), (json!({ "sidebar": "top" }), "off, left or right"), (json!({ "controls": { "teleport": true } }), "no control called"), (json!({ "title": "  " }), "`title`")] {
            let e = run(&mut b, "set_player", args).unwrap_err();
            assert!(e.contains(expect), "{e}");
        }
        assert!(INSTRUCTIONS.contains("title_is_missing") && INSTRUCTIONS.contains("not in order"));
    }

    #[test]
    fn text_with_symbols_the_fonts_cannot_draw_gets_a_warning() {
        let mut a = app_with(1);
        let r = run(&mut a, "add_shape", json!({ "slide": 1, "shape": "rounded_rectangle", "text": "\u{2302} Home" })).unwrap();
        assert!(r["warning"].as_str().unwrap().contains("\u{2302}"), "{r}");
        for ok in ["Home", "Next \u{2192}", "Caf\u{e9} \u{2013} 50% \u{2022} done", "\u{4e2d}\u{6587}"] {
            let r = run(&mut a, "add_text", json!({ "slide": 1, "text": ok })).unwrap();
            assert!(r.get("warning").is_none(), "{ok}: {r}");
        }
        let r = run(&mut a, "add_text", json!({ "slide": 1, "text": "Done \u{2705}" })).unwrap();
        assert!(r["warning"].is_string());
        let id = r["id"].as_str().unwrap().to_string();
        let r = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "text": "Done \u{1F44D}" })).unwrap();
        assert!(r["warning"].is_string(), "a change to the text is checked too");
        let r = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "text": "Done" })).unwrap();
        assert!(r.get("warning").is_none());
        assert!(INSTRUCTIONS.contains("plain words, not symbols") && INSTRUCTIONS.contains("add_icon"));
    }

    #[test]
    fn icons_are_found_by_name_and_added_as_vector_shapes_in_the_lessons_colors() {
        let mut a = app_with(1);
        let r = run(&mut a, "find_icons", json!({ "query": "house", "count": 5 })).unwrap();
        assert!(r["icons"].as_array().unwrap().iter().any(|n| n == "house"), "{r}");
        assert!(run(&mut a, "find_icons", json!({ "query": "zzzz-nothing" })).unwrap()["icons"].as_array().unwrap().is_empty());
        let r = run(&mut a, "add_icon", json!({ "slide": 1, "icon": "house", "size": 200 })).unwrap();
        assert_eq!((r["type"].as_str(), r["alt"].as_str()), (Some("vector shape"), Some("house")), "{r}");
        let o = a.project.as_ref().unwrap().slides[0].layers[0].objects.last().unwrap().clone();
        assert!(matches!(o.kind, Kind::Path(_)));
        assert_eq!((o.stroke.map(|s| s.color), o.fill), (Some(a.project.as_ref().unwrap().theme.text()), None), "lines in the lesson's text color unless told otherwise, nothing inside");
        let look = o.icon_look().expect("an icon from the library");
        assert_eq!((look.lines, look.inside, look.bold), (a.project.as_ref().unwrap().theme.text(), None, 0.0));
        assert!(o.icon.as_ref().is_some_and(|sil| !sil.is_empty()), "the solid inside is kept so it can be filled");
        assert!((o.w.max(o.h) - 200.0).abs() <= 1.0 && ((o.x + o.w / 2.0) - 960.0).abs() <= 1.0, "200 on its longest side, centred: {} {} {} {}", o.x, o.y, o.w, o.h);
        // Chosen color, place, and the words spelled however they are written.
        let r = run(&mut a, "add_icon", json!({ "slide": 1, "icon": "Arrow Right", "color": "#E87722", "x": 50, "y": 60, "alt": "Next slide" })).unwrap();
        assert_eq!((r["alt"].as_str(), r["x"].as_f64(), r["y"].as_f64()), (Some("Next slide"), Some(50.0), Some(60.0)));
        // The outline color colors the lines and the fill color the inside.
        run(&mut a, "add_icon", json!({ "slide": 1, "icon": "house", "color": "#C0392B", "fill": "#F9E79F" })).unwrap();
        let o = a.project.as_ref().unwrap().slides[0].layers[0].objects.last().unwrap().clone();
        let look = o.icon_look().unwrap();
        assert_eq!((look.lines.hex().as_str(), look.inside.map(|c| c.hex()).as_deref()), ("#C0392B", Some("#F9E79F")));
        let r = run(&mut a, "add_icon", json!({ "slide": 1, "icon": "star", "decorative": true })).unwrap();
        assert!(r["alt"].is_null() || r["alt"] == "", "{r}");
        assert!(a.project.as_ref().unwrap().slides[0].layers[0].objects.last().unwrap().decorative);
        // A name that is not an icon says what is close.
        let e = run(&mut a, "add_icon", json!({ "slide": 1, "icon": "hous" })).unwrap_err();
        assert!(e.contains("no icon called") && e.contains("house") || e.contains("find_icons"), "{e}");
        let e = run(&mut a, "add_icon", json!({ "slide": 1, "icon": "" })).unwrap_err();
        assert!(e.contains("`icon`"), "{e}");
        // One undo step takes an icon back.
        let n = a.project.as_ref().unwrap().slides[0].layers[0].objects.len();
        run(&mut a, "undo", json!({})).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layers[0].objects.len(), n - 1);
    }

    #[test]
    fn alt_text_can_be_set_and_is_questioned_on_an_object_that_has_text() {
        let mut a = app_with(1);
        let r = run(&mut a, "add_shape", json!({ "slide": 1, "shape": "ellipse", "name": "Dot", "alt": "A decorative dot", "decorative": false })).unwrap();
        assert_eq!(r["alt"], "A decorative dot");
        assert!(r.get("warning").is_none(), "a shape with no text can have alt text: {r}");
        let r = run(&mut a, "add_shape", json!({ "slide": 1, "shape": "rounded_rectangle", "text": "Next", "alt": "Next: Fire exits" })).unwrap();
        assert!(r["warning"].as_str().unwrap().contains("replaces that text"), "{r}");
        let id = r["id"].as_str().unwrap().to_string();
        let r = run(&mut a, "set_object", json!({ "slide": 1, "object": id, "alt": "" })).unwrap();
        assert!(r.get("alt").is_none() && r.get("warning").is_none(), "clearing it leaves the button's own text: {r}");
        let r = run(&mut a, "set_object", json!({ "slide": 1, "object": "Dot", "decorative": true })).unwrap();
        assert_eq!(r["decorative"], true);
        assert!(INSTRUCTIONS.contains("give no alt text to a button or text that has words on it"));
    }

    #[test]
    fn a_button_is_one_accessible_object_and_the_checker_is_available_to_the_ai() {
        let mut a = app_with(3);
        run(&mut a, "set_design", json!({ "colors": { "Accent 1": "#0B2545" } })).unwrap();
        let r = run(&mut a, "add_button", json!({ "slide": 1, "text": "Forklifts", "action": "go_to_slide", "target": 2, "icon": "truck" })).unwrap();
        assert_eq!(r["button"]["text"], "Forklifts");
        assert!(r["button"].get("alt").is_none(), "the words name it: no alt text");
        let p = a.project.as_ref().unwrap();
        let objs = &p.slides[0].layers[0].objects;
        assert_eq!(objs.len(), 2, "the button and its icon, and no separate text box");
        let (button, icon) = (&objs[0], &objs[1]);
        assert!(matches!(button.kind, Kind::Rect) && button.text == "Forklifts" && button.fill == Some(Rgba(11, 37, 69, 255)));
        assert_eq!(button.style.color, Rgba(255, 255, 255, 255), "white words on a dark button");
        assert!(icon.decorative && icon.alt.is_empty() && matches!(icon.kind, Kind::Path(_)), "the icon is skipped by screen readers");
        assert!(button.group.is_some() && button.group == icon.group, "they move together");
        assert!(button.margins[0] > 0.0, "the words sit beside the icon");
        let t: Vec<_> = p.slides[0].triggers.iter().collect();
        assert_eq!(t.len(), 1, "one click trigger, on the button");
        assert!(matches!((&t[0].owner, &t[0].event, &t[0].action), (Owner::Object(id), Event::Click, Action::JumpToSlide(to)) if *id == button.id && *to == p.slides[1].id));
        // A light button gets dark words.
        let r = run(&mut a, "add_button", json!({ "slide": 1, "text": "Next", "action": "next", "fill": "#F3F5F8" })).unwrap();
        assert_ne!(r["button"]["id"], Value::Null);
        let light = a.project.as_ref().unwrap().slides[0].layers[0].objects.iter().find(|o| o.text == "Next").unwrap().clone();
        assert_ne!(light.style.color, Rgba(255, 255, 255, 255), "dark words on a light button");
        // What the AI builds this way has nothing for the checker to complain about.
        let c = run(&mut a, "check_accessibility", json!({ "slide": 1 })).unwrap();
        assert_eq!(c["errors"], 0, "{c}");
        assert!(!c["issues"].as_array().unwrap().iter().any(|i| i["message"].as_str().unwrap().contains("sits on top")), "{c}");
        // The old way, a text box laid over a clickable shape, is reported with the fix.
        let shape = run(&mut a, "add_shape", json!({ "slide": 2, "shape": "rounded_rectangle", "name": "Card", "x": 100, "y": 100, "width": 400, "height": 200 })).unwrap();
        run(&mut a, "add_text", json!({ "slide": 2, "text": "Fire exits", "x": 120, "y": 150, "width": 360 })).unwrap();
        let sid = a.project.as_ref().unwrap().slides[1].id.clone();
        a.project.as_mut().unwrap().slides[1].triggers.push(Trigger { id: "tt".into(), owner: Owner::Object(shape["id"].as_str().unwrap().into()), event: Event::Click, action: Action::JumpToSlide(sid), conditions: vec![] });
        let c = run(&mut a, "check_accessibility", json!({})).unwrap();
        let msg = c["issues"].as_array().unwrap().iter().map(|i| i["message"].as_str().unwrap().to_string()).find(|m| m.contains("sits on top")).expect("reported");
        assert!(msg.contains("Put the words in"), "{msg}");
        assert_eq!(c["issues"].as_array().unwrap().iter().find(|i| i["message"].as_str().unwrap().contains("sits on top")).unwrap()["slide"], 2);
        // Mistakes in the call are explained.
        for (args, expect) in [
            (json!({ "slide": 1, "text": "Go", "action": "teleport" }), "go_to_slide"),
            (json!({ "slide": 1, "text": "Go", "action": "go_to_slide" }), "For go_to_slide"),
            (json!({ "slide": 1, "text": "  " }), "`text`"),
            (json!({ "slide": 1, "text": "Go", "action": "next", "icon": "not-an-icon" }), "no icon called"),
        ] {
            let e = run(&mut a, "add_button", args).unwrap_err();
            assert!(e.contains(expect), "{e}");
        }
        // One undo step takes a button back, icon and trigger too.
        let mut b = app_with(2);
        run(&mut b, "add_button", json!({ "slide": 1, "text": "Home", "action": "go_to_slide", "target": 2, "icon": "house" })).unwrap();
        assert_eq!((b.project.as_ref().unwrap().slides[0].layers[0].objects.len(), b.project.as_ref().unwrap().slides[0].triggers.len()), (2, 1));
        run(&mut b, "undo", json!({})).unwrap();
        assert_eq!((b.project.as_ref().unwrap().slides[0].layers[0].objects.len(), b.project.as_ref().unwrap().slides[0].triggers.len()), (0, 0));
        assert!(INSTRUCTIONS.contains("check_accessibility") && INSTRUCTIONS.contains("add_button"));
    }

    #[test]
    fn objects_are_grouped_into_one_control_with_one_trigger_and_checked_as_one() {
        let mut a = app_with(3);
        run(&mut a, "add_shape", json!({ "slide": 1, "shape": "rounded_rectangle", "name": "Card", "x": 100, "y": 100, "width": 500, "height": 260 })).unwrap();
        run(&mut a, "add_text", json!({ "slide": 1, "text": "Fire exits", "name": "Words", "x": 120, "y": 250, "width": 460, "color": "#FFFFFF" })).unwrap();
        run(&mut a, "add_icon", json!({ "slide": 1, "icon": "door", "name": "Door icon", "x": 280, "y": 120, "color": "#FFFFFF" })).unwrap();
        let r = run(&mut a, "group_objects", json!({ "slide": 1, "objects": ["Card", "Words", "Door icon"], "action": "go_to_slide", "target": 3, "name": "Fire exits card" })).unwrap();
        assert_eq!((r["clickable"].clone(), r["owner"].clone(), r["screen_readers_read"].clone()), (json!(true), json!("Card"), json!("Fire exits")), "{r}");
        assert_eq!(r["warnings"].as_array().unwrap().len(), 0);
        let p = a.project.as_ref().unwrap();
        let objs = &p.slides[0].layers[0].objects;
        let gid = objs[0].group.clone().unwrap();
        assert!(objs.iter().all(|o| o.group.as_ref() == Some(&gid)), "all three share the group");
        assert_eq!(p.slides[0].layers[0].group_names.get(&gid).map(String::as_str), Some("Fire exits card"));
        assert_eq!(p.slides[0].triggers.len(), 1, "one trigger, on the first object");
        assert!(matches!(&p.slides[0].triggers[0].owner, Owner::Object(id) if *id == objs[0].id));
        // The checker sees one control: no words-over-a-button warning, no unnamed picture or icon.
        let c = run(&mut a, "check_accessibility", json!({ "slide": 1 })).unwrap();
        assert!(!c["issues"].as_array().unwrap().iter().any(|i| i["message"].as_str().unwrap().contains("sits on top") || i["message"].as_str().unwrap().contains("no alt text")), "{c}");
        // Alt text of its own replaces the words.
        let mut b = app_with(2);
        run(&mut b, "add_shape", json!({ "slide": 1, "shape": "ellipse", "name": "Dot" })).unwrap();
        run(&mut b, "add_icon", json!({ "slide": 1, "icon": "star", "name": "Star" })).unwrap();
        let r = run(&mut b, "group_objects", json!({ "slide": 1, "objects": ["Dot", "Star"], "action": "next", "alt": "Go to the next slide" })).unwrap();
        assert_eq!(r["screen_readers_read"], "Go to the next slide");
        // No words and no alt: it is flagged as the AI builds it.
        let mut c2 = app_with(2);
        run(&mut c2, "add_shape", json!({ "slide": 1, "shape": "ellipse", "name": "Dot" })).unwrap();
        run(&mut c2, "add_icon", json!({ "slide": 1, "icon": "star", "name": "Star", "decorative": true })).unwrap();
        let r = run(&mut c2, "group_objects", json!({ "slide": 1, "objects": ["Dot", "Star"], "action": "next" })).unwrap();
        assert!(r["warnings"][0].as_str().unwrap().contains("cannot name it"), "{r}");
        // A group of its own accord, no click: just moves together.
        let r = run(&mut c2, "group_objects", json!({ "slide": 2, "objects": ["Dot", "Star"] }));
        assert!(r.is_err(), "those objects are on slide 1");
        // Mistakes are explained, and nothing changes.
        let before = c2.undo.len();
        for (args, expect) in [
            (json!({ "slide": 1, "objects": ["Dot"] }), "at least two"),
            (json!({ "slide": 1, "objects": ["Dot", "Missing"] }), "no object"),
            (json!({ "slide": 1, "objects": ["Dot", "Dot"] }), "listed twice"),
            (json!({ "slide": 1, "objects": ["Dot", "Star"], "action": "go_to_slide" }), "For go_to_slide"),
            (json!({ "slide": 1, "objects": ["Dot", "Star"], "action": "fly" }), "go_to_slide"),
        ] {
            let e = run(&mut c2, "group_objects", args).unwrap_err();
            assert!(e.contains(expect), "{e}");
        }
        assert_eq!(c2.undo.len(), before);
        // A group that is already together cannot be split by grouping one of its parts with something else.
        run(&mut c2, "add_text", json!({ "slide": 1, "text": "Extra", "name": "Extra" })).unwrap();
        let e = run(&mut c2, "group_objects", json!({ "slide": 1, "objects": ["Dot", "Extra"] })).unwrap_err();
        assert!(e.contains("already in a group"), "{e}");
        // One undo step takes the grouping and its trigger back.
        run(&mut a, "undo", json!({})).unwrap();
        let p = a.project.as_ref().unwrap();
        assert!(p.slides[0].layers[0].objects.iter().all(|o| o.group.is_none()) && p.slides[0].triggers.is_empty());
        assert!(INSTRUCTIONS.contains("group_objects"));
    }

    #[test]
    fn a_layout_of_the_same_name_is_replaced_and_deleting_one_detaches_its_slides() {
        let mut a = app_with(1);
        let first = run(&mut a, "create_layout", title_and_content()).unwrap();
        let again = run(&mut a, "create_layout", json!({ "name": "TITLE AND CONTENT", "placeholders": [{ "kind": "title", "x": 100, "y": 100, "width": 800, "height": 200 }] })).unwrap();
        assert_eq!((again["id"].clone(), again["replaced_one_of_the_same_name"].as_bool()), (first["id"].clone(), Some(true)));
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), 2, "blank, and the one that was replaced");
        run(&mut a, "apply_layout", json!({ "slide": 1, "layout": first["id"] })).unwrap();
        assert_eq!(a.project.as_ref().unwrap().slides[0].layout.as_deref(), first["id"].as_str());
        let r = run(&mut a, "delete_layout", json!({ "layout": "Title and Content" })).unwrap();
        assert_eq!(r["slides_detached"], 1);
        assert!(a.project.as_ref().unwrap().slides[0].layout.is_none());
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), 1);
        let e = run(&mut a, "apply_layout", json!({ "slide": 1, "layout": "Gone" })).unwrap_err();
        assert!(e.contains("There is no layout") && e.contains("Blank"), "{e}");
        assert!(run(&mut a, "add_slide", json!({ "layout": "Gone" })).unwrap_err().contains("There is no layout"));
        assert_eq!(a.project.as_ref().unwrap().slides.len(), 1, "a refused add_slide adds nothing");
    }

    #[test]
    fn a_layout_that_does_not_fit_or_is_incomplete_is_refused() {
        let mut a = app_with(1);
        for (args, expect) in [
            (json!({ "name": "", "placeholders": [] }), "`name` must be"),
            (json!({ "name": "Empty", "placeholders": [] }), "at least one"),
            (json!({ "name": "Off", "placeholders": [{ "kind": "Heading 2", "x": 1800, "y": 0, "width": 400, "height": 100 }] }), "does not fit"),
            (json!({ "name": "Kind", "placeholders": [{ "kind": "Banner", "x": 0, "y": 0, "width": 100, "height": 100 }] }), "`kind` must be"),
            (json!({ "name": "NoSize", "placeholders": [{ "kind": "Heading 2", "x": 0, "y": 0 }] }), "`width` is needed"),
            (json!({ "name": "Shape", "placeholders": [], "shapes": [{ "x": 0, "y": 0, "width": 5000, "height": 10 }] }), "does not fit"),
            (json!({ "name": "Color", "placeholders": [{ "kind": "Heading 2", "x": 0, "y": 0, "width": 100, "height": 100, "color": "zzz" }] }), "not a color"),
        ] {
            let e = run(&mut a, "create_layout", args).unwrap_err();
            assert!(e.contains(expect), "{e}");
        }
        assert_eq!(a.project.as_ref().unwrap().layouts.len(), 1, "nothing was added");
        assert!(a.undo.is_empty());
    }

    #[test]
    fn the_instructions_tell_a_tool_to_set_the_design_up_before_building() {
        for word in ["get_design", "set_design", "create_layout", "add_slide", "placeholders", "brand", "palette"] { assert!(INSTRUCTIONS.contains(word), "the instructions should mention {word}"); }
        let t = tools();
        for name in ["get_design", "set_design", "create_layout", "delete_layout", "apply_layout"] { assert!(t.iter().any(|d| d.name == name), "{name}"); }
        assert_eq!(access("get_design"), Some(Access::Read));
        assert_eq!(access("create_layout"), Some(Access::Edit));
    }
}
