//! A script as a node graph, in the style of Blueprints: what the canvas
//! draws.
//!
//! The graph is another view of the same reading as the blocks, not a second
//! model: each handler is an event node, its statements a chain of nodes
//! joined by execution wires, and each call or operator a value node wired
//! into the pin it feeds; simple arithmetic over names and literals stays on
//! its pin as a formula. Numbers, text and switches stay on their pins,
//! edited in place through [`super::edit`] as the blocks are, and names show
//! on their pins. Compound statements branch: an If has True, False and Next,
//! a loop Body and Next, a Match one output per arm and Next.
//!
//! Every node has a stable [`Node::key`] (its place in the code) and a
//! [`Node::print`] (a fingerprint of its text), which is how a position stored
//! beside the script finds its node again after the script was edited
//! elsewhere ([`super::layout`]).

use serde::Serialize;

use super::{Block, Slot, Span, VisualScript};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub wires: Vec<Wire>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Node {
    /// Where it is in the code: `Wick.update/2` (the third statement of
    /// `update`), `Wick.update/2.0/1` (the second of its first nested chain),
    /// `Wick.update/2:to` (the value feeding its `to` pin).
    pub key: String,
    /// A fingerprint of its text, which survives it moving in the code.
    pub print: String,
    /// `event`, `function`, `values`, `statement` or `value`.
    pub kind: &'static str,
    pub title: String,
    /// The comment lines written above it.
    pub note: Option<String>,
    /// Pins on the left: what it takes. The execution pin in is not listed;
    /// every node but an event, a function and the values has one.
    pub inputs: Vec<Pin>,
    /// Pins on the right: where execution goes next, by name. A value node's
    /// one output is not listed.
    pub outputs: Vec<String>,
    /// Where it is on the canvas, in points, left and top.
    pub at: [f32; 2],
    /// Whether [`Self::at`] was stored, rather than laid out.
    pub placed: bool,
    /// Where it is in the script, for a code view to find it.
    pub span: Span,
}

/// An input pin: its name, and what it holds, unless a wire feeds it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pin {
    pub name: String,
    pub value: Option<Slot>,
}

/// A wire: execution from a node's named output to the next node, or a value
/// node's result into a named input.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wire {
    pub from: String,
    /// The output it leaves by; empty for a value node's result.
    pub out: String,
    pub to: String,
    /// The input it enters by; empty for execution in.
    pub into: String,
    pub flow: bool,
}

/// `script` as a graph, laid out (positions from [`super::layout`] are laid
/// over it by the caller). `source` is the script's text, for fingerprints.
#[must_use]
pub fn graph(script: &VisualScript, source: &str) -> Graph {
    let mut builder = Builder {
        source,
        graph: Graph {
            nodes: Vec::new(),
            wires: Vec::new(),
        },
    };
    for container in &script.containers {
        if !container.fields.is_empty() {
            let inputs = container
                .fields
                .iter()
                .map(|field| Pin {
                    name: field.name.clone(),
                    value: field.value.clone(),
                })
                .collect();
            builder.node(Node {
                key: format!("{}.values", container.name),
                print: print("values"),
                kind: "values",
                title: format!("{} keeps", container.name),
                note: None,
                inputs,
                outputs: Vec::new(),
                at: [0.0, 0.0],
                placed: false,
                span: container.span,
            });
        }
        for handler in &container.handlers {
            let key = format!("{}.{}", container.name, handler.name);
            let title = if handler.params.is_empty() {
                handler.label.clone()
            } else {
                format!("{} ({})", handler.label, handler.params.join(", "))
            };
            builder.node(Node {
                key: key.clone(),
                print: print(&key),
                kind: if handler.role == "event" {
                    "event"
                } else {
                    "function"
                },
                title,
                note: handler.note.clone(),
                inputs: Vec::new(),
                outputs: vec![String::new()],
                at: [0.0, 0.0],
                placed: false,
                span: handler.span,
            });
            builder.chain(&handler.body, &key, (&key, ""));
        }
    }
    super::layout::arrange(&mut builder.graph);
    builder.graph
}

struct Builder<'a> {
    source: &'a str,
    graph: Graph,
}

impl Builder<'_> {
    fn node(&mut self, node: Node) {
        self.graph.nodes.push(node);
    }

    fn flow(&mut self, from: (&str, &str), to: &str) {
        self.graph.wires.push(Wire {
            from: from.0.to_owned(),
            out: from.1.to_owned(),
            to: to.to_owned(),
            into: String::new(),
            flow: true,
        });
    }

    /// `blocks` as a chain of nodes keyed under `path`, entered from `from`
    /// (a node's key and the output execution leaves it by). Comments become
    /// the note of the node after them.
    fn chain(&mut self, blocks: &[Block], path: &str, from: (&str, &str)) {
        let mut from = (from.0.to_owned(), from.1.to_owned());
        let mut note: Option<String> = None;
        let mut index = 0;
        for block in blocks {
            if let Block::Comment { text, .. } = block {
                note = Some(match note.take() {
                    Some(above) => format!("{above}\n{text}"),
                    None => text.clone(),
                });
                continue;
            }
            let key = format!("{path}/{index}");
            index += 1;
            self.statement(block, &key, note.take());
            self.flow((&from.0, &from.1), &key);
            let continues = matches!(
                block,
                Block::If { .. } | Block::While { .. } | Block::For { .. } | Block::Match { .. }
            );
            from = (
                key,
                if continues {
                    "Next".to_owned()
                } else {
                    String::new()
                },
            );
        }
    }

    fn statement(&mut self, block: &Block, key: &str, above: Option<String>) {
        let span = block_span(block);
        let shape = shape(block, self.source, span);
        let mut node = Node {
            key: key.to_owned(),
            print: print(self.source.get(span[0]..span[1]).unwrap_or_default()),
            kind: "statement",
            title: shape.title,
            note: above,
            inputs: Vec::new(),
            outputs: shape.outputs,
            at: [0.0, 0.0],
            placed: false,
            span,
        };
        for (name, value) in shape.inputs {
            self.input(&mut node, &name, &value);
        }
        self.node(node);
        for (index, (output, blocks)) in shape.nested.into_iter().enumerate() {
            // By position, not by label: a Switch's labels are its patterns,
            // which can hold anything a pattern can.
            let path = format!("{key}.{index}");
            self.chain(blocks, &path, (key, &output));
        }
    }

    /// An input pin named `name` taking `value`: on the pin when it is a
    /// literal, a name or code, and from a value node otherwise.
    fn input(&mut self, node: &mut Node, name: &str, value: &Slot) {
        let key = format!(
            "{}:{}",
            node.key,
            if name.is_empty() { "value" } else { name }
        );
        match value {
            // Arithmetic and logic over names and literals reads best as the
            // formula it is, on the pin, as Unreal's Math Expression node
            // does: `this.grace - dt`, not three nodes. Most of a long
            // script's value nodes were exactly these.
            Slot::Op { span, .. } | Slot::Unary { span, .. } if formula(value) => {
                node.inputs.push(Pin {
                    name: name.to_owned(),
                    value: Some(Slot::Code {
                        source: self
                            .source
                            .get(span[0]..span[1])
                            .unwrap_or_default()
                            .to_owned(),
                        span: *span,
                    }),
                });
            }
            Slot::Call { .. } | Slot::Op { .. } | Slot::Unary { .. } => {
                self.value(value, &key);
                node.inputs.push(Pin {
                    name: name.to_owned(),
                    value: None,
                });
                self.graph.wires.push(Wire {
                    from: key,
                    out: String::new(),
                    to: node.key.clone(),
                    into: name.to_owned(),
                    flow: false,
                });
            }
            literal => node.inputs.push(Pin {
                name: name.to_owned(),
                value: Some(literal.clone()),
            }),
        }
    }

    /// A value node for a call, an operator or a negation, keyed `key`.
    fn value(&mut self, value: &Slot, key: &str) {
        let span = value.span();
        let (title, pins): (String, Vec<(String, &Slot)>) = match value {
            Slot::Call { callee, args, .. } => (
                callee.clone(),
                args.iter()
                    .enumerate()
                    .map(|(index, arg)| ((index + 1).to_string(), arg))
                    .collect(),
            ),
            Slot::Op {
                op, left, right, ..
            } => (
                (*op).to_owned(),
                vec![("a".to_owned(), &**left), ("b".to_owned(), &**right)],
            ),
            Slot::Unary { op, operand, .. } => (
                if *op == "!" { "not" } else { "negate" }.to_owned(),
                vec![(String::new(), &**operand)],
            ),
            _ => (String::new(), Vec::new()),
        };
        let mut node = Node {
            key: key.to_owned(),
            print: print(self.source.get(span[0]..span[1]).unwrap_or_default()),
            kind: "value",
            title,
            note: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            at: [0.0, 0.0],
            placed: false,
            span,
        };
        for (name, pin) in pins {
            self.input(&mut node, &name, pin);
        }
        self.node(node);
    }
}

/// How a statement is drawn: its title, what its pins take, where execution
/// can leave it, and the chains its outputs run.
struct Shape<'a> {
    title: String,
    inputs: Vec<(String, Slot)>,
    outputs: Vec<String>,
    nested: Vec<(String, &'a [Block])>,
}

impl<'a> Shape<'a> {
    /// A statement execution passes straight through.
    fn through(title: String, inputs: Vec<(String, Slot)>) -> Self {
        Self {
            title,
            inputs,
            outputs: vec![String::new()],
            nested: Vec::new(),
        }
    }

    /// A statement execution stops at.
    fn end(title: &str, inputs: Vec<(String, Slot)>) -> Self {
        Self {
            title: title.to_owned(),
            inputs,
            outputs: Vec::new(),
            nested: Vec::new(),
        }
    }

    /// A statement whose outputs run `nested` chains, then Next.
    fn branching(
        title: String,
        input: (String, Slot),
        nested: Vec<(String, &'a [Block])>,
        outputs: Vec<String>,
    ) -> Self {
        let mut outputs = outputs;
        outputs.push("Next".to_owned());
        Self {
            title,
            inputs: vec![input],
            outputs,
            nested,
        }
    }
}

fn shape<'a>(block: &'a Block, source: &str, span: Span) -> Shape<'a> {
    let pin = |name: &str, value: &Slot| (name.to_owned(), value.clone());
    match block {
        Block::Set { target, value, .. } => Shape::through(
            format!("Set {}", name_of(target, source)),
            vec![pin("to", value)],
        ),
        Block::Change {
            target, op, value, ..
        } => {
            let verb = match *op {
                "+=" => "Add to",
                "-=" => "Take from",
                "*=" => "Multiply",
                "/=" => "Divide",
                _ => "Change",
            };
            Shape::through(
                format!("{verb} {}", name_of(target, source)),
                vec![pin("by", value)],
            )
        }
        Block::Let {
            name,
            mutable,
            value,
            ..
        } => Shape::through(
            format!("{} {name}", if *mutable { "Var" } else { "Let" }),
            value.iter().map(|value| pin("=", value)).collect(),
        ),
        Block::Do {
            call: Slot::Call { callee, args, .. },
            ..
        } => Shape::through(
            callee.clone(),
            args.iter()
                .enumerate()
                .map(|(index, arg)| pin(&(index + 1).to_string(), arg))
                .collect(),
        ),
        Block::Do { call, .. } => Shape::through("Do".to_owned(), vec![pin("", call)]),
        Block::If {
            condition,
            then,
            otherwise,
            ..
        } => {
            let mut nested = vec![("True".to_owned(), then.as_slice())];
            if let Some(otherwise) = otherwise {
                nested.push(("False".to_owned(), otherwise.as_slice()));
            }
            let outputs = vec!["True".to_owned(), "False".to_owned()];
            Shape::branching("Branch".to_owned(), pin("if", condition), nested, outputs)
        }
        Block::While {
            condition, body, ..
        } => Shape::branching(
            "While".to_owned(),
            pin("while", condition),
            vec![("Body".to_owned(), body.as_slice())],
            vec!["Body".to_owned()],
        ),
        Block::For {
            name, items, body, ..
        } => Shape::branching(
            format!("For each {name}"),
            pin("in", items),
            vec![("Body".to_owned(), body.as_slice())],
            vec!["Body".to_owned()],
        ),
        Block::Match { subject, arms, .. } => {
            let labels: Vec<String> = arms.iter().map(|arm| arm.patterns.join(" or ")).collect();
            let nested = labels
                .iter()
                .cloned()
                .zip(arms)
                .map(|(label, arm)| (label, arm.body.as_slice()))
                .collect();
            Shape::branching("Switch".to_owned(), pin("on", subject), nested, labels)
        }
        Block::Return { value, .. } => Shape::end(
            "Give back",
            value.iter().map(|value| pin("", value)).collect(),
        ),
        Block::Break { .. } => Shape::end("Stop the loop", Vec::new()),
        Block::Continue { .. } => Shape::end("Next round", Vec::new()),
        Block::Code { source: code, .. } | Block::Comment { text: code, .. } => Shape::through(
            "Code".to_owned(),
            vec![(
                String::new(),
                Slot::Code {
                    source: code.clone(),
                    span,
                },
            )],
        ),
    }
}

/// Whether `slot` is arithmetic or logic over names and literals only, with
/// no call inside: what is shown as a formula on its pin.
fn formula(slot: &Slot) -> bool {
    match slot {
        Slot::Op { left, right, .. } => formula(left) && formula(right),
        Slot::Unary { operand, .. } => formula(operand),
        Slot::Number { .. }
        | Slot::Text { .. }
        | Slot::Bool { .. }
        | Slot::Null { .. }
        | Slot::Name { .. } => true,
        Slot::Call { .. } | Slot::Code { .. } => false,
    }
}

fn name_of(target: &Slot, source: &str) -> String {
    match target {
        Slot::Name { path, .. } => path.clone(),
        other => {
            let span = other.span();
            source.get(span[0]..span[1]).unwrap_or_default().to_owned()
        }
    }
}

const fn block_span(block: &Block) -> Span {
    match block {
        Block::Set { span, .. }
        | Block::Change { span, .. }
        | Block::Let { span, .. }
        | Block::Do { span, .. }
        | Block::If { span, .. }
        | Block::While { span, .. }
        | Block::For { span, .. }
        | Block::Match { span, .. }
        | Block::Return { span, .. }
        | Block::Break { span }
        | Block::Continue { span }
        | Block::Code { span, .. }
        | Block::Comment { span, .. } => *span,
    }
}

/// A fingerprint of `text` that ignores spacing: 64-bit FNV-1a, in hex.
pub(super) fn print(text: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.split_whitespace().flat_map(str::bytes) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}
