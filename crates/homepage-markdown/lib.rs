use std::{borrow::Cow, fs, io, ops::Deref, path::Path};

use homepage_traits::ReproduceTokens;
use markdown::{CompileOptions, Constructs, LineEnding, Options, ParseOptions, mdast::Node};
use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize, Default, ReproduceTokens)]
#[serde(rename_all = "kebab-case")]
pub enum Variant {
    #[default]
    Normal,
    Music,
}

impl Variant {
    pub fn is_music(&self) -> bool {
        matches!(self, Self::Music)
    }
}

#[derive(Deserialize, ReproduceTokens)]
pub struct Preamble {
    pub title: Cow<'static, str>,
    #[serde(rename = "pubDate")]
    pub publication_date: Cow<'static, str>,
    pub description: Cow<'static, str>,

    #[serde(default)]
    pub authors: Cow<'static, [Cow<'static, str>]>,
    #[serde(default)]
    pub reviewers: Cow<'static, [Cow<'static, str>]>,
    #[serde(default)]
    pub tags: Cow<'static, [Cow<'static, str>]>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub time: Option<Cow<'static, str>>,

    #[serde(default)]
    pub variant: Variant,

    #[serde(default = "default_true")]
    pub ligatures: bool,
}

fn default_true() -> bool {
    true
}

#[derive(ReproduceTokens)]
pub struct BlogPost {
    pub preamble: Preamble,
    pub templatable_source: Cow<'static, str>,
    pub templatable_description: Cow<'static, str>,

    pub slug: Cow<'static, str>,

    pub filepath: Cow<'static, str>,
}

impl Deref for BlogPost {
    type Target = Preamble;

    fn deref(&self) -> &Self::Target {
        &self.preamble
    }
}

#[derive(Debug, Error)]
pub enum FromFileError {
    #[error("io error: {0:?}")]
    Io(#[from] io::Error),

    #[error("markdown parsing error:\n{0:?}")]
    MarkdownParse(String),

    #[error("markdown compilation error:\n{0:?}")]
    MarkdownCompile(String),

    #[error("no preamble")]
    NoPreamble,

    #[error("parse preamble: {0:?}")]
    ParsePreamble(#[from] serde_yaml::Error),
}

fn find_preamble(ast: Node) -> Option<String> {
    let Node::Root(root) = ast else {
        return None;
    };

    let mut yaml_contents = None;

    for child in root.children {
        if let Node::Yaml(yaml) = child {
            yaml_contents = Some(yaml.value);
            break;
        }
    }

    Some(yaml_contents?)
}

fn replace_template_char(
    mut input: &str,
    delimiters: (&str, &str),
    mapping: &mut Vec<String>,
) -> String {
    let mut res = String::new();
    while let Some((before, after)) = input.split_once(delimiters.0) {
        res.push_str(before);

        let Some((middle, rest)) = after.split_once(delimiters.1) else {
            res.push_str(delimiters.0);
            res.push_str(after);
            break;
        };

        res.push_str(&format!("TEMPLATEMAPPING{}X", mapping.len()));
        mapping.push(format!("{}{}{}", delimiters.0, middle, delimiters.1));
        input = rest;
    }

    res.push_str(input);

    res
}

fn replace_templates(input: &str) -> (String, Vec<String>) {
    let mut mapping = Vec::new();

    let res = replace_template_char(input, ("{{", "}}"), &mut mapping);
    let res = replace_template_char(&res, ("{%", "%}"), &mut mapping);

    (res, mapping)
}

fn put_back_templates(mut input: String, substitutions: Vec<String>) -> String {
    for (idx, text) in substitutions.into_iter().enumerate() {
        // use replacen 1 to stop search early
        input = input.replacen(&format!("TEMPLATEMAPPING{idx}X"), &text, 1);
    }

    input
}

impl BlogPost {
    pub fn from_file(
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> Result<BlogPost, FromFileError> {
        let parse_options = ParseOptions {
            constructs: Constructs {
                attention: true,
                autolink: true,
                block_quote: true,
                character_escape: true,
                character_reference: true,
                code_indented: false,
                code_fenced: true,
                code_text: true,
                definition: true,
                frontmatter: true,
                gfm_autolink_literal: true,
                gfm_footnote_definition: true,
                gfm_label_start_footnote: true,
                gfm_strikethrough: true,
                gfm_table: true,
                gfm_task_list_item: true,
                hard_break_escape: true,
                hard_break_trailing: true,
                heading_atx: true,
                heading_setext: true,
                html_flow: true,
                html_text: true,
                label_start_image: true,
                label_start_link: true,
                label_end: true,
                list_item: true,
                math_flow: false,
                math_text: false,
                mdx_esm: false,
                mdx_expression_flow: false,
                mdx_expression_text: false,
                mdx_jsx_flow: false,
                mdx_jsx_text: false,
                thematic_break: true,
            },
            gfm_strikethrough_single_tilde: true,
            math_text_single_dollar: false,
            mdx_expression_parse: None,
            mdx_esm_parse: None,
        };

        let compile_options = CompileOptions {
            allow_any_img_src: false,
            allow_dangerous_html: true,
            allow_dangerous_protocol: false,
            default_line_ending: LineEnding::LineFeed,
            gfm_footnote_back_label: Some("↩".to_string()),
            gfm_footnote_clobber_prefix: Some("footnote-".to_string()),
            gfm_footnote_label_attributes: None,
            gfm_footnote_label_tag_name: None,
            gfm_footnote_label: None,
            gfm_task_list_item_checkable: true,
            gfm_tagfilter: false,
        };

        let contents = fs::read_to_string(&path)?;
        let (contents, contents_subs) = replace_templates(&contents);

        let ast = markdown::to_mdast(&contents, &parse_options)
            .map_err(|i| FromFileError::MarkdownParse(i.to_string()))?;

        let preamble = find_preamble(ast).ok_or(FromFileError::NoPreamble)?;
        let preamble: Preamble = serde_yaml::from_str(&preamble)?;

        let (description, description_subs) = replace_templates(&preamble.description);

        let options = Options {
            parse: parse_options,
            compile: compile_options,
        };

        let html = markdown::to_html_with_options(&contents, &options)
            .map_err(|i| FromFileError::MarkdownCompile(i.to_string()))?;

        let description_html = markdown::to_html_with_options(&description, &options)
            .map_err(|i| FromFileError::MarkdownCompile(i.to_string()))?;

        let html = put_back_templates(html, contents_subs);
        let description_html = put_back_templates(description_html, description_subs);

        let slug = path
            .as_ref()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .replace(" ", "-")
            .replace(|i: char| !i.is_ascii_alphanumeric() && i != '-', "");

        let filepath = path
            .as_ref()
            .strip_prefix(root.as_ref())
            .unwrap()
            .to_string_lossy()
            .into_owned();

        Ok(BlogPost {
            slug: slug.into(),
            preamble,
            templatable_source: format!("{description_html}{html}").into(),
            templatable_description: description_html.into(),
            filepath: filepath.into(),
        })
    }
}
