// compiler-rs/diagnostics/src/catalog.rs

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use fon_parser::{Member, MemberId, Value};
use infra::{Diagnostic, DiagnosticArg, DiagnosticValue, MessageId, Severity, Span};

/// A language or script identifier used by the diagnostic catalog.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Locale(String);

impl Locale {
    /// Create a locale identifier from its normalized spelling.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Return the locale identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One parsed catalog template component.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TemplatePart {
    Text(String),
    Placeholder(String),
}

/// A validated message template for one locale.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MessageTemplate {
    parts: Vec<TemplatePart>,
}

/// All locale translations for one message ID.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalizedMessage {
    translations: BTreeMap<Locale, MessageTemplate>,
}

/// An immutable catalog loaded from a FON language-pack document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    messages: BTreeMap<String, LocalizedMessage>,
}

/// A rendered diagnostic retaining its machine-readable identity and primary span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedDiagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub primary: Span,
    pub message: String,
    pub labels: Vec<RenderedLabel>,
    pub notes: Vec<String>,
    pub suggestions: Vec<infra::DiagnosticSuggestion>,
}

/// A rendered secondary label retaining its source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedLabel {
    pub span: Span,
    pub message: String,
}

/// Errors found while loading or validating a language catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogError {
    Parse { message: String },
    InvalidRoot,
    InvalidMessage { id: String },
    DuplicateMessage { id: String },
    InvalidTranslation { id: String, locale: String },
    MissingDefaultLocale { id: String },
    DuplicateTranslation { id: String, locale: String },
    InvalidPlaceholder { id: String, placeholder: String },
}

/// Errors found while rendering one structured diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    MissingMessage { id: String },
    MissingArgument { id: String, argument: String },
}

impl Catalog {
    /// Load the compiler's built-in language catalog once per process.
    pub fn embedded() -> Result<Arc<Self>, CatalogError> {
        static EMBEDDED: OnceLock<Result<Arc<Catalog>, CatalogError>> = OnceLock::new();
        EMBEDDED
            .get_or_init(|| {
                Self::from_source(include_str!("../../../locale/locales.fon")).map(Arc::new)
            })
            .clone()
    }

    /// Parse and validate a nested FON language catalog.
    pub fn from_source(source: &str) -> Result<Self, CatalogError> {
        let result = fon_parser::parse(source);
        if let Some(diagnostic) = result.diagnostics.first() {
            return Err(CatalogError::Parse {
                message: format!("{}: {}", diagnostic.code, diagnostic.message),
            });
        }

        let Some(member_ids) = result.document.ast.object_members() else {
            return Err(CatalogError::InvalidRoot);
        };
        let mut messages = BTreeMap::new();
        for member_id in member_ids.iter().copied() {
            collect_message_namespace(
                &result.document.ast,
                member_id,
                String::new(),
                &mut messages,
            )?;
        }
        validate_default_locales(&messages)?;
        Ok(Self { messages })
    }

    /// Return true when the catalog contains a message ID.
    pub fn contains(&self, id: MessageId) -> bool {
        self.messages.contains_key(id.as_str())
    }

    /// Render a message ID with typed arguments and locale fallback.
    pub fn render_message(
        &self,
        id: MessageId,
        args: &[DiagnosticArg],
        requested_locale: Locale,
    ) -> Result<String, RenderError> {
        let key = id.as_str();
        let Some(message) = self.messages.get(key) else {
            return Err(RenderError::MissingMessage { id: key.to_owned() });
        };
        let template = select_template(message, &requested_locale);
        render_template(key, template, args)
    }

    /// Render one diagnostic using exact, parent, then English locale fallback.
    pub fn render(
        &self,
        diagnostic: &Diagnostic,
        requested_locale: Locale,
    ) -> Result<RenderedDiagnostic, RenderError> {
        let message = self.render_message(
            diagnostic.message_id,
            &diagnostic.args,
            requested_locale.clone(),
        )?;
        let labels = diagnostic
            .labels
            .iter()
            .map(|label| {
                self.render_message(label.message_id, &label.args, requested_locale.clone())
                    .map(|message| RenderedLabel {
                        span: label.span,
                        message,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let notes = diagnostic
            .notes
            .iter()
            .map(|note| self.render_message(note.message_id, &note.args, requested_locale.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(RenderedDiagnostic {
            severity: diagnostic.severity,
            code: diagnostic.code,
            primary: diagnostic.primary,
            message,
            labels,
            notes,
            suggestions: diagnostic.suggestions.clone(),
        })
    }
}

/// Collect one nested message namespace and flatten its path in the immutable catalog.
fn collect_message_namespace(
    ast: &fon_parser::ast::Ast,
    member_id: MemberId,
    prefix: String,
    messages: &mut BTreeMap<String, LocalizedMessage>,
) -> Result<(), CatalogError> {
    let Some(Member::Binding(binding)) = ast.member(member_id) else {
        return Err(CatalogError::InvalidMessage { id: prefix });
    };
    let path = append_path(&prefix, binding.key.raw.as_str());
    let Some(Value::Object(object)) = ast.value(binding.value) else {
        return Err(CatalogError::InvalidMessage { id: path });
    };

    let mut translations = BTreeMap::new();
    for child_id in object.members.iter().copied() {
        let Some(Member::Binding(child)) = ast.member(child_id) else {
            return Err(CatalogError::InvalidMessage { id: path.clone() });
        };
        let child_path = append_path(&path, child.key.raw.as_str());
        match ast.value(child.value) {
            Some(Value::String(string)) if is_locale(child.key.raw.as_str()) => {
                let template = parse_template(&path, child.key.raw.as_str(), &string.raw)?;
                if translations
                    .insert(Locale::new(child.key.raw.clone()), template)
                    .is_some()
                {
                    return Err(CatalogError::DuplicateTranslation {
                        id: path.clone(),
                        locale: child.key.raw.clone(),
                    });
                }
            }
            Some(Value::Object(_)) => {
                collect_message_namespace(ast, child_id, path.clone(), messages)?;
            }
            Some(Value::String(_)) => {
                return Err(CatalogError::InvalidTranslation {
                    id: path.clone(),
                    locale: child.key.raw.clone(),
                });
            }
            Some(_) | None => {
                return Err(CatalogError::InvalidTranslation {
                    id: child_path,
                    locale: child.key.raw.clone(),
                });
            }
        }
    }

    if !translations.is_empty()
        && messages
            .insert(path.clone(), LocalizedMessage { translations })
            .is_some()
    {
        return Err(CatalogError::DuplicateMessage { id: path });
    }
    Ok(())
}

/// Validate that every leaf message has the required English translation.
fn validate_default_locales(
    messages: &BTreeMap<String, LocalizedMessage>,
) -> Result<(), CatalogError> {
    for (id, message) in messages {
        if !message.translations.contains_key(&Locale::new("en")) {
            return Err(CatalogError::MissingDefaultLocale { id: id.clone() });
        }
    }
    Ok(())
}

/// Parse a backtick string into text and named placeholder parts.
fn parse_template(id: &str, locale: &str, raw: &str) -> Result<MessageTemplate, CatalogError> {
    let Some(content) = raw
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
    else {
        return Err(CatalogError::InvalidTranslation {
            id: id.to_owned(),
            locale: locale.to_owned(),
        });
    };
    let mut parts = Vec::new();
    let mut cursor = 0;
    while let Some(relative_open) = content[cursor..].find('{') {
        let open = cursor + relative_open;
        if open > cursor {
            parts.push(TemplatePart::Text(content[cursor..open].to_owned()));
        }
        let Some(relative_close) = content[open + 1..].find('}') else {
            return Err(CatalogError::InvalidPlaceholder {
                id: id.to_owned(),
                placeholder: content[open + 1..].to_owned(),
            });
        };
        let close = open + 1 + relative_close;
        let placeholder = &content[open + 1..close];
        if !is_placeholder(placeholder) {
            return Err(CatalogError::InvalidPlaceholder {
                id: id.to_owned(),
                placeholder: placeholder.to_owned(),
            });
        }
        parts.push(TemplatePart::Placeholder(placeholder.to_owned()));
        cursor = close + 1;
    }
    if cursor < content.len() {
        parts.push(TemplatePart::Text(content[cursor..].to_owned()));
    }
    if parts.is_empty() {
        parts.push(TemplatePart::Text(content.to_owned()));
    }
    Ok(MessageTemplate { parts })
}

/// Select an exact locale, its language parent, or the English default.
fn select_template<'a>(message: &'a LocalizedMessage, locale: &Locale) -> &'a MessageTemplate {
    message
        .translations
        .get(locale)
        .or_else(|| {
            locale
                .as_str()
                .split_once('-')
                .and_then(|(parent, _)| message.translations.get(&Locale::new(parent)))
        })
        .or_else(|| message.translations.get(&Locale::new("en")))
        .expect("validated catalogs always contain the English locale")
}

/// Render all template parts using named diagnostic arguments.
fn render_template(
    id: &str,
    template: &MessageTemplate,
    args: &[DiagnosticArg],
) -> Result<String, RenderError> {
    let mut output = String::new();
    for part in &template.parts {
        match part {
            TemplatePart::Text(text) => output.push_str(text),
            TemplatePart::Placeholder(name) => {
                let Some(argument) = args.iter().find(|argument| argument.name == *name) else {
                    return Err(RenderError::MissingArgument {
                        id: id.to_owned(),
                        argument: name.clone(),
                    });
                };
                output.push_str(&display_value(&argument.value));
            }
        }
    }
    Ok(output)
}

/// Convert a typed diagnostic argument into its localized interpolation text.
fn display_value(value: &DiagnosticValue) -> String {
    match value {
        DiagnosticValue::Text(value)
        | DiagnosticValue::Identifier(value)
        | DiagnosticValue::Type(value) => value.clone(),
        DiagnosticValue::Integer(value) => value.to_string(),
        DiagnosticValue::Unsigned(value) => value.to_string(),
        DiagnosticValue::Boolean(value) => value.to_string(),
    }
}

/// Join a nested object path while preserving stable dotted message IDs internally.
fn append_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_owned()
    } else {
        format!("{prefix}.{segment}")
    }
}

/// Check the deliberately narrow placeholder grammar used by catalog templates.
fn is_placeholder(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Check locale identifiers accepted by the language-only catalog format.
fn is_locale(value: &str) -> bool {
    let mut segments = value.split('-');
    let Some(language) = segments.next() else {
        return false;
    };
    let language_valid =
        (2..=3).contains(&language.len()) && language.bytes().all(|byte| byte.is_ascii_lowercase());
    language_valid
        && segments
            .all(|script| script.len() == 4 && script.bytes().all(|byte| byte.is_ascii_lowercase()))
}
