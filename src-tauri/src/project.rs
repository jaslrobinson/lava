use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerType {
    Text,
    Shape,
    Image,
    Group,
    Stack,
    Overlap,
    Progress,
    Fonticon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeKind {
    Rectangle,
    Circle,
    Oval,
    Triangle,
    Arc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnchorPoint {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationTrigger {
    Time,
    Scroll,
    Reactive,
    Tap,
    Show,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationType {
    Fade,
    Rotate,
    Scale,
    Translate,
    Color,
    Blur,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EasingType {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Bounce,
    Elastic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalVarType {
    Text,
    Number,
    Color,
    Switch,
    List,
}

/// A value that can be either a number or a string (formula).
/// Many KLWP properties accept both literal numbers and formula strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

/// A value that can be a boolean or a string (formula).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrString {
    Bool(bool),
    String(String),
}

/// A value that can be a string, number, or boolean.
/// Used for global variable values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GlobalVarValue {
    String(String),
    Number(f64),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: GlobalVarType,
    pub value: GlobalVarValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shadow {
    pub color: String,
    pub dx: f64,
    pub dy: f64,
    pub radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationLoop {
    None,
    Restart,
    Reverse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Animation {
    #[serde(rename = "type")]
    pub animation_type: AnimationType,
    pub trigger: AnimationTrigger,
    pub rule: String,
    pub amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub easing: Option<EasingType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<f64>,
    #[serde(rename = "loop")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_mode: Option<AnimationLoop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaleMode {
    Fit,
    Fill,
    Crop,
    Stretch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProgressStyle {
    Arc,
    Bar,
    Circle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerProperties {
    pub x: NumberOrString,
    pub y: NumberOrString,
    pub width: NumberOrString,
    pub height: NumberOrString,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_x: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_y: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<AnchorPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<BoolOrString>,

    // Text properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<TextAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,

    // Shape properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_kind: Option<ShapeKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,

    // Image properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_mode: Option<ScaleMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,

    // Progress properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ProgressStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_color: Option<String>,

    // FontIcon properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_set: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glyph_code: Option<String>,

    // Stack/Group properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<Orientation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub layer_type: LayerType,
    pub properties: LayerProperties,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animations: Option<Vec<Animation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Layer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundType {
    Color,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Background {
    #[serde(rename = "type")]
    pub bg_type: BackgroundType,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub version: String,
    pub name: String,
    pub resolution: Resolution,
    pub background: Background,
    pub globals: Vec<GlobalVariable>,
    pub layers: Vec<Layer>,
}

