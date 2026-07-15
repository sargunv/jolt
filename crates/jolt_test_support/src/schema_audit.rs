use std::fmt::{Debug, Write};
use std::marker::PhantomData;

use jolt_syntax::schema::{
    Cardinality, Disambiguation, Matcher, NodeClass, Repeat, SlotShape, SyntaxSchema,
    TrailingSeparator,
};
use jolt_syntax::{Language, SyntaxElement, SyntaxNode, SyntaxSlot};

/// Corpus-only audit of declarative syntax shapes. The matcher intentionally
/// lives in test support: its search is bounded by each fixture node's direct
/// child count and is not the production syntax-factory algorithm.
pub struct SchemaAudit<K> {
    language: &'static str,
    representation: SchemaRepresentation,
    files: usize,
    diagnostic_files: usize,
    nodes: usize,
    exact: usize,
    malformed: usize,
    clean_missing: Vec<String>,
    diagnostic_missing: Vec<String>,
    clean_unexpected: Vec<String>,
    diagnostic_unexpected: Vec<String>,
    owners: Vec<String>,
    kind: PhantomData<K>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SchemaRepresentation {
    ExhaustiveSlots,
    RawChildren,
}

impl<K: Copy + Eq + Debug> SchemaAudit<K> {
    #[must_use]
    pub fn new(language: &'static str) -> Self {
        Self::with_representation(language, SchemaRepresentation::ExhaustiveSlots)
    }

    /// Audits a parser that still stores constructed and list fields inline in
    /// its owner's raw child sequence.
    #[must_use]
    pub fn new_raw(language: &'static str) -> Self {
        Self::with_representation(language, SchemaRepresentation::RawChildren)
    }

    fn with_representation(language: &'static str, representation: SchemaRepresentation) -> Self {
        Self {
            language,
            representation,
            files: 0,
            diagnostic_files: 0,
            nodes: 0,
            exact: 0,
            malformed: 0,
            clean_missing: Vec::new(),
            diagnostic_missing: Vec::new(),
            clean_unexpected: Vec::new(),
            diagnostic_unexpected: Vec::new(),
            owners: Vec::new(),
            kind: PhantomData,
        }
    }

    pub fn visit<L, C>(
        &mut self,
        schema: &SyntaxSchema<K, C>,
        label: &str,
        root: SyntaxNode<'_, L>,
        has_diagnostics: bool,
    ) where
        L: Language<Kind = K>,
        C: Copy + Eq,
    {
        self.files += 1;
        self.diagnostic_files += usize::from(has_diagnostics);
        self.visit_node(schema, label, root, has_diagnostics, &mut Vec::new());
    }

    fn visit_node<L, C>(
        &mut self,
        schema: &SyntaxSchema<K, C>,
        label: &str,
        node: SyntaxNode<'_, L>,
        has_diagnostics: bool,
        occurrences: &mut Vec<(K, usize)>,
    ) where
        L: Language<Kind = K>,
        C: Copy + Eq,
    {
        self.nodes += 1;
        let occurrence = if let Some((_, count)) = occurrences
            .iter_mut()
            .find(|(kind, _)| *kind == node.kind())
        {
            *count += 1;
            *count
        } else {
            occurrences.push((node.kind(), 1));
            1
        };
        let node_label = format!("{label}: {:?} occurrence {occurrence}", node.kind());
        for index in 0..node.slot_count() {
            if let Some(SyntaxSlot::Node(child)) = node.slot_at(index) {
                assert_eq!(
                    child.parent(),
                    Some(node),
                    "incorrect physical parent for {node_label} slot {index}"
                );
                assert_eq!(
                    child.index(),
                    index,
                    "incorrect physical parent slot for {node_label} slot {index}"
                );
            }
        }
        if node.kind() == L::error_node_kind() {
            let owner = node.parent().map_or_else(
                || "<root>".to_owned(),
                |parent| format!("{:?}", parent.kind()),
            );
            self.owners.push(format!("{node_label} owned by {owner}"));
        }

        let children = node.children_with_tokens().collect::<Vec<_>>();
        match schema.nodes.iter().find(|shape| shape.kind == node.kind()) {
            None => unexpected_for(self, has_diagnostics)
                .push(format!("{node_label} has no declared shape")),
            Some(shape) => {
                if shape.class == NodeClass::Malformed {
                    self.malformed += 1;
                    for child in node.children() {
                        self.visit_node(schema, label, child, has_diagnostics, occurrences);
                    }
                    return;
                }
                if node.is_directly_malformed() {
                    let range = node.text_range();
                    unexpected_for(self, has_diagnostics).push(format!(
                        "{node_label} has valid kind with direct malformed ownership \
                         [bytes={}, tokens={}, children={}] ({})",
                        range.len().get(),
                        node.tokens().count(),
                        children.len(),
                        render_children(&children)
                    ));
                    for child in node.children() {
                        self.visit_node(schema, label, child, has_diagnostics, occurrences);
                    }
                    return;
                }
                let exact = match_slots(schema, shape.slots, &children, false, self.representation);
                let exact_paths = path_count(&exact, children.len(), false);
                assert!(
                    exact_paths < 2,
                    "ambiguous schema match for {node_label} ({})",
                    render_children(&children)
                );
                if exact_paths == 1 {
                    self.exact += 1;
                } else {
                    let allowing_missing =
                        match_slots(schema, shape.slots, &children, true, self.representation);
                    let detail = format!("{node_label} ({})", render_children(&children));
                    let missing_paths = allowing_missing
                        .iter()
                        .filter(|state| state.position == children.len() && state.missing)
                        .map(|state| state.paths)
                        .max()
                        .unwrap_or(0);
                    assert!(
                        missing_paths < 2,
                        "ambiguous missing-slot ownership for {node_label} ({})",
                        render_children(&children)
                    );
                    if missing_paths == 1 {
                        missing_for(self, has_diagnostics).push(detail);
                    } else {
                        unexpected_for(self, has_diagnostics).push(detail);
                    }
                }
            }
        }
        for child in node.children() {
            self.visit_node(schema, label, child, has_diagnostics, occurrences);
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(output, "language = {}", self.language).unwrap();
        writeln!(output, "fixture_files = {}", self.files).unwrap();
        writeln!(
            output,
            "diagnostic_fixture_files = {}",
            self.diagnostic_files
        )
        .unwrap();
        writeln!(output, "audited_nodes = {}", self.nodes).unwrap();
        writeln!(output, "exact_valid_shapes = {}", self.exact).unwrap();
        writeln!(output, "malformed_nodes = {}", self.malformed).unwrap();
        writeln!(
            output,
            "clean_missing_required_shapes = {}",
            self.clean_missing.len()
        )
        .unwrap();
        writeln!(
            output,
            "diagnostic_missing_required_shapes = {}",
            self.diagnostic_missing.len()
        )
        .unwrap();
        writeln!(
            output,
            "clean_unexpected_shapes = {}",
            self.clean_unexpected.len()
        )
        .unwrap();
        writeln!(
            output,
            "diagnostic_unexpected_shapes = {}",
            self.diagnostic_unexpected.len()
        )
        .unwrap();
        writeln!(output, "error_node_owners = {}", self.owners.len()).unwrap();
        output.push_str("\n[clean_missing_required]\n");
        for item in &self.clean_missing {
            writeln!(output, "{item}").unwrap();
        }
        output.push_str("\n[diagnostic_missing_required]\n");
        for item in &self.diagnostic_missing {
            writeln!(output, "{item}").unwrap();
        }
        output.push_str("\n[clean_unexpected]\n");
        for item in &self.clean_unexpected {
            writeln!(output, "{item}").unwrap();
        }
        output.push_str("\n[diagnostic_unexpected]\n");
        for item in &self.diagnostic_unexpected {
            writeln!(output, "{item}").unwrap();
        }
        output.push_str("\n[error_node_ownership]\n");
        for item in &self.owners {
            writeln!(output, "{item}").unwrap();
        }
        output
    }
}

fn missing_for<K>(audit: &mut SchemaAudit<K>, has_diagnostics: bool) -> &mut Vec<String> {
    if has_diagnostics {
        &mut audit.diagnostic_missing
    } else {
        &mut audit.clean_missing
    }
}

fn unexpected_for<K>(audit: &mut SchemaAudit<K>, has_diagnostics: bool) -> &mut Vec<String> {
    if has_diagnostics {
        &mut audit.diagnostic_unexpected
    } else {
        &mut audit.clean_unexpected
    }
}

fn render_children<L: Language>(children: &[SyntaxElement<'_, L>]) -> String
where
    L::Kind: Debug,
{
    children
        .iter()
        .map(|child| match child {
            SyntaxElement::Node(node) => format!("{:?}", node.kind()),
            SyntaxElement::Token(token) => format!("{:?}({:?})", token.kind(), token.text()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Copy)]
struct State {
    position: usize,
    missing: bool,
    paths: u8,
}

fn add_state(states: &mut Vec<State>, position: usize, missing: bool, paths: u8) {
    if let Some(state) = states
        .iter_mut()
        .find(|state| state.position == position && state.missing == missing)
    {
        state.paths = state.paths.saturating_add(paths).min(2);
    } else {
        states.push(State {
            position,
            missing,
            paths,
        });
    }
}

fn path_count(states: &[State], position: usize, missing: bool) -> u8 {
    states
        .iter()
        .find(|state| state.position == position && state.missing == missing)
        .map_or(0, |state| state.paths)
}

fn match_slots<L, C>(
    schema: &SyntaxSchema<L::Kind, C>,
    slots: &[SlotShape<L::Kind, C>],
    children: &[SyntaxElement<'_, L>],
    allow_missing: bool,
    representation: SchemaRepresentation,
) -> Vec<State>
where
    L: Language,
    C: Copy + Eq,
{
    let mut states = vec![State {
        position: 0,
        missing: false,
        paths: 1,
    }];
    for slot in slots {
        let mut next = Vec::new();
        for state in states {
            let ends = slot_ends(schema, *slot, children, state.position, representation);
            let constructed_at_position = ends.iter().any(|(end, paths)| {
                *end == state.position && *paths > 0 && matches!(slot.matcher, Matcher::List(_))
            });
            let owns_available_match = slot.disambiguation == Disambiguation::LeftmostLongest
                && ends.iter().any(|(_, paths)| *paths > 0);
            let owns_recovery = representation == SchemaRepresentation::ExhaustiveSlots
                && slot.cardinality != Cardinality::Optional
                && children.get(state.position).is_some_and(
                    |child| matches!(child, SyntaxElement::Node(node) if node.is_directly_malformed()),
                );
            for (end, paths) in ends {
                add_state(
                    &mut next,
                    end,
                    state.missing,
                    state.paths.saturating_mul(paths).min(2),
                );
            }
            if allow_missing
                && slot_required(*slot)
                && !constructed_at_position
                && !owns_available_match
                && !owns_recovery
            {
                add_state(&mut next, state.position, true, state.paths);
            }
        }
        states = next;
    }
    states
}

fn slot_required<K: Copy, C: Copy>(slot: SlotShape<K, C>) -> bool {
    match slot.repeat {
        Repeat::Separated { minimum, .. } => minimum > 0,
        Repeat::None => matches!(
            slot.cardinality,
            Cardinality::Required | Cardinality::OneOrMore
        ),
    }
}

fn slot_ends<L, C>(
    schema: &SyntaxSchema<L::Kind, C>,
    slot: SlotShape<L::Kind, C>,
    children: &[SyntaxElement<'_, L>],
    start: usize,
    representation: SchemaRepresentation,
) -> Vec<(usize, u8)>
where
    L: Language,
    C: Copy + Eq,
{
    let mut ends = match slot.repeat {
        Repeat::None => repeated(
            schema,
            slot.matcher,
            slot.cardinality,
            slot.disambiguation,
            children,
            start,
            representation,
        ),
        Repeat::Separated {
            separator,
            minimum,
            trailing,
            ..
        } => separated(
            schema,
            slot.matcher,
            separator,
            minimum,
            trailing,
            slot.disambiguation,
            children,
            start,
            representation,
        ),
    };
    if slot.disambiguation == Disambiguation::LeftmostLongest
        && let Some(longest) = ends.iter().map(|(end, _)| *end).max()
    {
        ends.clear();
        ends.push((longest, 1));
    }
    ends
}

fn add_end(ends: &mut Vec<(usize, u8)>, end: usize, paths: u8) {
    if let Some((_, count)) = ends.iter_mut().find(|(position, _)| *position == end) {
        *count = count.saturating_add(paths).min(2);
    } else {
        ends.push((end, paths));
    }
}
fn merge_ends(ends: &mut Vec<(usize, u8)>) {
    let old = std::mem::take(ends);
    for (end, paths) in old {
        add_end(ends, end, paths);
    }
}

fn repeated<L, C>(
    schema: &SyntaxSchema<L::Kind, C>,
    matcher: Matcher<L::Kind, C>,
    cardinality: Cardinality,
    policy: Disambiguation,
    children: &[SyntaxElement<'_, L>],
    start: usize,
    representation: SchemaRepresentation,
) -> Vec<(usize, u8)>
where
    L: Language,
    C: Copy + Eq,
{
    let (minimum, maximum) = match cardinality {
        Cardinality::Required => (1, 1),
        Cardinality::Optional => (0, 1),
        Cardinality::Many => (0, children.len().saturating_sub(start)),
        Cardinality::OneOrMore => (1, children.len().saturating_sub(start)),
    };
    let mut results = Vec::new();
    if minimum == 0 {
        results.push((start, 1));
    }
    let mut frontier = vec![(start, 1_u8)];
    for count in 1..=maximum {
        let mut next = Vec::new();
        for (position, paths) in frontier {
            for (end, matched) in matcher_ends(
                schema,
                matcher,
                policy,
                children,
                position,
                cardinality != Cardinality::Optional,
                representation,
            ) {
                if end != position || matches!(matcher, Matcher::List(_)) {
                    add_end(&mut next, end, paths.saturating_mul(matched).min(2));
                }
            }
        }
        if next.is_empty() {
            break;
        }
        if count >= minimum {
            for &(end, paths) in &next {
                add_end(&mut results, end, paths);
            }
        }
        frontier = next;
    }
    results
}

#[allow(clippy::too_many_arguments)]
fn separated<L, C>(
    schema: &SyntaxSchema<L::Kind, C>,
    element: Matcher<L::Kind, C>,
    separator: Matcher<L::Kind, C>,
    minimum: u16,
    trailing: TrailingSeparator,
    policy: Disambiguation,
    children: &[SyntaxElement<'_, L>],
    start: usize,
    representation: SchemaRepresentation,
) -> Vec<(usize, u8)>
where
    L: Language,
    C: Copy + Eq,
{
    let mut results = Vec::new();
    if minimum == 0 && trailing != TrailingSeparator::Required {
        results.push((start, 1));
    }
    let mut elements = matcher_ends(
        schema,
        element,
        policy,
        children,
        start,
        true,
        representation,
    );
    let mut count = 1_u16;
    while !elements.is_empty() {
        if count >= minimum && trailing != TrailingSeparator::Required {
            for &(end, paths) in &elements {
                add_end(&mut results, end, paths);
            }
        }
        let mut separators = Vec::new();
        for (position, paths) in elements {
            for (end, matched) in matcher_ends(
                schema,
                separator,
                policy,
                children,
                position,
                false,
                representation,
            ) {
                if end != position {
                    add_end(&mut separators, end, paths.saturating_mul(matched).min(2));
                }
            }
        }
        if count >= minimum && trailing != TrailingSeparator::Forbidden {
            for &(end, paths) in &separators {
                add_end(&mut results, end, paths);
            }
        }
        let mut next = Vec::new();
        for (position, paths) in separators {
            for (end, matched) in matcher_ends(
                schema,
                element,
                policy,
                children,
                position,
                true,
                representation,
            ) {
                if end != position {
                    add_end(&mut next, end, paths.saturating_mul(matched).min(2));
                }
            }
        }
        elements = next;
        count = count.saturating_add(1);
    }
    results
}

fn matcher_ends<L, C>(
    schema: &SyntaxSchema<L::Kind, C>,
    matcher: Matcher<L::Kind, C>,
    policy: Disambiguation,
    children: &[SyntaxElement<'_, L>],
    start: usize,
    accept_recovery: bool,
    representation: SchemaRepresentation,
) -> Vec<(usize, u8)>
where
    L: Language,
    C: Copy + Eq,
{
    if representation == SchemaRepresentation::ExhaustiveSlots
        && accept_recovery
        && children.get(start).is_some_and(
            |child| matches!(child, SyntaxElement::Node(node) if node.is_directly_malformed()),
        )
    {
        return vec![(start + 1, 1)];
    }
    match matcher {
        Matcher::Constructed(kind) | Matcher::List(kind) => {
            if let Some(SyntaxElement::Node(node)) = children.get(start)
                && node.kind() == kind
            {
                return vec![(start + 1, 1)];
            }
            if representation == SchemaRepresentation::ExhaustiveSlots {
                return Vec::new();
            }

            let shape = schema
                .nodes
                .iter()
                .find(|shape| shape.kind == kind)
                .expect("constructed matcher must reference a declared node");
            if matches!(matcher, Matcher::List(_)) {
                assert_eq!(shape.class, NodeClass::List);
            } else {
                assert_eq!(shape.class, NodeClass::Valid);
            }
            match_slots(
                schema,
                shape.slots,
                &children[start..],
                false,
                representation,
            )
            .into_iter()
            .filter(|state| !state.missing)
            .map(|state| (start + state.position, state.paths))
            .collect()
        }
        Matcher::Choice(choices) => {
            if policy == Disambiguation::LongestThenFirst {
                let mut selected = Vec::new();
                let mut longest = None;
                for choice in choices {
                    let ends = matcher_ends(
                        schema,
                        *choice,
                        policy,
                        children,
                        start,
                        accept_recovery,
                        representation,
                    );
                    let choice_longest = ends.iter().map(|(end, _)| *end).max();
                    if choice_longest > longest {
                        longest = choice_longest;
                        selected = ends;
                    }
                }
                selected.retain(|(end, _)| Some(*end) == longest);
                merge_ends(&mut selected);
                selected
            } else {
                let mut results = Vec::new();
                for choice in choices {
                    for (end, paths) in matcher_ends(
                        schema,
                        *choice,
                        policy,
                        children,
                        start,
                        accept_recovery,
                        representation,
                    ) {
                        add_end(&mut results, end, paths);
                    }
                }
                results
            }
        }
        _ => children
            .get(start)
            .filter(|child| accepts(schema, matcher, **child, representation))
            .map_or_else(Vec::new, |_| vec![(start + 1, 1)]),
    }
}

fn category_shape<K: Copy + Eq, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    category: C,
) -> &jolt_syntax::schema::CategoryShape<K, C> {
    schema
        .categories
        .iter()
        .find(|shape| shape.category == category)
        .expect("declared category")
}
fn accepts<L, C>(
    schema: &SyntaxSchema<L::Kind, C>,
    matcher: Matcher<L::Kind, C>,
    child: SyntaxElement<'_, L>,
    representation: SchemaRepresentation,
) -> bool
where
    L: Language,
    C: Copy + Eq,
{
    match (matcher, child) {
        (Matcher::Token(kind), SyntaxElement::Token(token)) => token.kind() == kind,
        (Matcher::TokenSet(kinds) | Matcher::ElementSet(kinds), SyntaxElement::Token(token)) => {
            kinds.contains(&token.kind())
        }
        (Matcher::ElementSet(kinds) | Matcher::NodeSet(kinds), SyntaxElement::Node(node)) => {
            kinds.contains(&node.kind())
        }
        (Matcher::Contextual { kind, text }, SyntaxElement::Token(token)) => {
            token.kind() == kind && token.text() == text
        }
        (
            Matcher::Node(kind) | Matcher::Constructed(kind) | Matcher::List(kind),
            SyntaxElement::Node(node),
        ) => node.kind() == kind,
        (Matcher::Category(category), SyntaxElement::Node(node)) => {
            let category = category_shape(schema, category);
            (representation == SchemaRepresentation::ExhaustiveSlots
                && node.kind() == category.bogus)
                || category.kinds.contains(&node.kind())
        }
        (Matcher::AnyNode, SyntaxElement::Node(_)) | (Matcher::AnyElement, _) => true,
        _ => false,
    }
}

/// Checks every declared matcher, including unobserved corpus branches.
pub fn assert_schema_deterministic<K, C>(schema: &SyntaxSchema<K, C>)
where
    K: Copy + Eq + Debug,
    C: Copy + Eq + Debug,
{
    let mut errors = Vec::new();
    validate_inventory(schema, &mut errors);
    for node in schema.nodes {
        validate_node_storage(*node, &mut errors);
        validate_node_matchers(schema, *node, &mut errors);
    }
    assert!(
        errors.is_empty(),
        "schema determinism errors:\n{}",
        errors.join("\n")
    );
}

fn validate_inventory<K, C>(schema: &SyntaxSchema<K, C>, errors: &mut Vec<String>)
where
    K: Copy + Eq + Debug,
    C: Copy + Eq + Debug,
{
    for (index, node) in schema.nodes.iter().enumerate() {
        if schema.nodes[..index]
            .iter()
            .any(|other| other.kind == node.kind)
        {
            errors.push(format!("duplicate node kind {:?}", node.kind));
        }
    }
    for (index, category) in schema.categories.iter().enumerate() {
        if schema.categories[..index]
            .iter()
            .any(|other| other.category == category.category)
        {
            errors.push(format!("duplicate category {:?}", category.category));
        }
        for (member_index, member) in category.kinds.iter().enumerate() {
            if category.kinds[..member_index].contains(member) {
                errors.push(format!(
                    "category {:?} repeats member {member:?}",
                    category.category
                ));
            }
            match node_class(schema, *member) {
                Some(NodeClass::Valid) => {}
                _ => errors.push(format!(
                    "category {:?} contains non-valid node {member:?}",
                    category.category
                )),
            }
        }
        if category.kinds.contains(&category.bogus)
            || node_class(schema, category.bogus) != Some(NodeClass::Malformed)
        {
            errors.push(format!(
                "category {:?} has invalid bogus owner {:?}",
                category.category, category.bogus
            ));
        }
    }
}

fn validate_node_storage<K: Copy + Debug, C: Copy>(
    node: jolt_syntax::schema::NodeShape<K, C>,
    errors: &mut Vec<String>,
) {
    match node.class {
        NodeClass::Valid => {
            for (index, slot) in node.slots.iter().enumerate() {
                if !matches!(
                    slot.cardinality,
                    Cardinality::Required | Cardinality::Optional
                ) || !matches!(slot.repeat, Repeat::None)
                    || !single_target(slot.matcher)
                {
                    errors.push(format!(
                        "{:?}[{index}]: valid node field is not one fixed target slot",
                        node.kind
                    ));
                }
            }
        }
        NodeClass::List => {
            if node.slots.len() != 1
                || !matches!(
                    node.slots[0].cardinality,
                    Cardinality::Many | Cardinality::OneOrMore
                )
                || !single_target(node.slots[0].matcher)
                || matches!(
                    node.slots[0].repeat,
                    Repeat::Separated { separator, .. } if !single_target(separator)
                )
            {
                errors.push(format!(
                    "{:?}: list node must declare one variable entries role",
                    node.kind
                ));
            }
        }
        NodeClass::Malformed => {}
    }
}

fn validate_node_matchers<K, C>(
    schema: &SyntaxSchema<K, C>,
    node: jolt_syntax::schema::NodeShape<K, C>,
    errors: &mut Vec<String>,
) where
    K: Copy + Eq + Debug,
    C: Copy + Eq,
{
    for (index, slot) in node.slots.iter().enumerate() {
        validate_matcher(
            schema,
            slot.matcher,
            slot.disambiguation,
            node.kind,
            index,
            errors,
        );
        if nullable(slot.matcher) && !matches!(slot.matcher, Matcher::List(_)) {
            errors.push(format!("{:?}[{index}]: nullable slot matcher", node.kind));
        }
        if slot.cardinality == Cardinality::Optional
            && matches!(slot.matcher, Matcher::List(_))
            && nullable(slot.matcher)
        {
            errors.push(format!(
                "{:?}[{index}]: optional empty list has two absence representations",
                node.kind
            ));
        }
        if let Repeat::Separated {
            separator,
            minimum,
            trailing,
            ..
        } = slot.repeat
        {
            if nullable(separator) {
                errors.push(format!("{:?}[{index}]: nullable separator", node.kind));
            }
            if overlap(schema, slot.matcher, separator) {
                errors.push(format!(
                    "{:?}[{index}]: element/separator FIRST overlap",
                    node.kind
                ));
            }
            let mut boundary = match trailing {
                TrailingSeparator::Forbidden => first(schema, separator),
                TrailingSeparator::Optional => {
                    let mut set = first(schema, separator);
                    set.extend(first(schema, slot.matcher));
                    set
                }
                TrailingSeparator::Required => first(schema, slot.matcher),
            };
            if minimum == 0 {
                boundary.extend(first(schema, slot.matcher));
            }
            check_follow(
                schema,
                node.kind,
                index,
                slot.disambiguation,
                &boundary,
                &node.slots[index + 1..],
                errors,
            );
        } else if nullable(slot.matcher)
            || matches!(
                slot.cardinality,
                Cardinality::Optional | Cardinality::Many | Cardinality::OneOrMore
            )
        {
            check_follow(
                schema,
                node.kind,
                index,
                slot.disambiguation,
                &first(schema, slot.matcher),
                &node.slots[index + 1..],
                errors,
            );
        }
        if matches!(slot.cardinality, Cardinality::Many | Cardinality::OneOrMore)
            && nullable(slot.matcher)
        {
            errors.push(format!(
                "{:?}[{index}]: nullable repeated matcher",
                node.kind
            ));
        }
    }
}

fn single_target<K: Copy, C: Copy>(matcher: Matcher<K, C>) -> bool {
    match matcher {
        Matcher::Choice(choices) => choices.iter().copied().all(single_target),
        Matcher::Token(_)
        | Matcher::TokenSet(_)
        | Matcher::ElementSet(_)
        | Matcher::Contextual { .. }
        | Matcher::Node(_)
        | Matcher::Constructed(_)
        | Matcher::List(_)
        | Matcher::NodeSet(_)
        | Matcher::Category(_)
        | Matcher::AnyNode
        | Matcher::AnyElement => true,
    }
}

fn check_follow<K: Copy + Eq + Debug, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    kind: K,
    index: usize,
    policy: Disambiguation,
    boundary: &First<K>,
    following: &[SlotShape<K, C>],
    errors: &mut Vec<String>,
) {
    if let Some(follow) = following_first(schema, following)
        && sets_overlap(schema, boundary, &follow)
        && policy != Disambiguation::LeftmostLongest
    {
        errors.push(format!(
            "{kind:?}[{index}]: repeat boundary FIRST/FOLLOW overlap without leftmost_longest"
        ));
    }
}
fn validate_matcher<K: Copy + Eq + Debug, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    matcher: Matcher<K, C>,
    policy: Disambiguation,
    kind: K,
    index: usize,
    errors: &mut Vec<String>,
) {
    match matcher {
        Matcher::Token(kind) | Matcher::Contextual { kind, .. } => {
            if is_node_kind(schema, kind) {
                errors.push(format!(
                    "{kind:?}[{index}]: token matcher references a node kind"
                ));
            }
        }
        Matcher::TokenSet(kinds) => {
            for kind in kinds {
                if is_node_kind(schema, *kind) {
                    errors.push(format!("{kind:?}[{index}]: token set contains a node kind"));
                }
            }
        }
        Matcher::Node(target) => {
            if node_class(schema, target) != Some(NodeClass::Valid) {
                errors.push(format!(
                    "{kind:?}[{index}]: node matcher requires a valid node, got {target:?}"
                ));
            }
        }
        Matcher::Constructed(target) => {
            if node_class(schema, target) != Some(NodeClass::Valid) {
                errors.push(format!(
                    "{kind:?}[{index}]: constructed matcher requires a valid node"
                ));
            }
        }
        Matcher::List(target) => {
            if node_class(schema, target) != Some(NodeClass::List) {
                errors.push(format!(
                    "{kind:?}[{index}]: list matcher requires a list node"
                ));
            }
        }
        Matcher::NodeSet(kinds) => {
            for target in kinds {
                if node_class(schema, *target) != Some(NodeClass::Valid) {
                    errors.push(format!(
                        "{kind:?}[{index}]: node set contains non-valid node {target:?}"
                    ));
                }
            }
        }
        Matcher::ElementSet(kinds) => {
            for target in kinds {
                if is_node_kind(schema, *target)
                    && node_class(schema, *target) != Some(NodeClass::Valid)
                {
                    errors.push(format!(
                        "{kind:?}[{index}]: element set contains non-valid node {target:?}"
                    ));
                }
            }
        }
        Matcher::Category(category) => {
            if !schema
                .categories
                .iter()
                .any(|shape| shape.category == category)
            {
                errors.push(format!("{kind:?}[{index}]: unknown category"));
            }
        }
        Matcher::Choice(choices) => {
            if choices.is_empty() {
                errors.push(format!("{kind:?}[{index}]: empty choice"));
            }
            for (left_index, left) in choices.iter().enumerate() {
                for right in &choices[left_index + 1..] {
                    if (nullable(*left) && nullable(*right) || overlap(schema, *left, *right))
                        && policy != Disambiguation::LongestThenFirst
                    {
                        errors.push(format!(
                            "{kind:?}[{index}]: choice tie without longest_then_first"
                        ));
                    }
                }
                validate_matcher(schema, *left, policy, kind, index, errors);
            }
        }
        Matcher::AnyNode | Matcher::AnyElement => {
            if node_class(schema, kind) == Some(NodeClass::Valid) {
                errors.push(format!(
                    "{kind:?}[{index}]: valid field cannot accept an arbitrary recovery element"
                ));
            }
        }
    }
}

fn is_node_kind<K: Copy + Eq, C>(schema: &SyntaxSchema<K, C>, kind: K) -> bool {
    schema.nodes.iter().any(|node| node.kind == kind)
}

fn node_class<K: Copy + Eq, C>(schema: &SyntaxSchema<K, C>, kind: K) -> Option<NodeClass> {
    schema
        .nodes
        .iter()
        .find(|node| node.kind == kind)
        .map(|node| node.class)
}

#[derive(Clone)]
struct First<K> {
    kinds: Vec<K>,
    contextual: Vec<(K, &'static str)>,
    any_node: bool,
    any_element: bool,
}
impl<K> Default for First<K> {
    fn default() -> Self {
        Self {
            kinds: Vec::new(),
            contextual: Vec::new(),
            any_node: false,
            any_element: false,
        }
    }
}
impl<K: Copy + Eq> First<K> {
    fn insert(&mut self, kind: K) {
        if !self.kinds.contains(&kind) {
            self.kinds.push(kind);
        }
    }
    fn extend(&mut self, other: Self) {
        for kind in other.kinds {
            self.insert(kind);
        }
        for contextual in other.contextual {
            if !self.contextual.contains(&contextual) {
                self.contextual.push(contextual);
            }
        }
        self.any_node |= other.any_node;
        self.any_element |= other.any_element;
    }
    fn empty(&self) -> bool {
        self.kinds.is_empty() && self.contextual.is_empty() && !self.any_node && !self.any_element
    }
}
fn first<K: Copy + Eq, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    matcher: Matcher<K, C>,
) -> First<K> {
    let mut set = First::default();
    match matcher {
        Matcher::Token(k) | Matcher::Node(k) | Matcher::Constructed(k) | Matcher::List(k) => {
            set.insert(k);
        }
        Matcher::Contextual { kind, text } => set.contextual.push((kind, text)),
        Matcher::TokenSet(k) | Matcher::NodeSet(k) | Matcher::ElementSet(k) => {
            for x in k {
                set.insert(*x);
            }
        }
        Matcher::Category(c) => {
            let category = category_shape(schema, c);
            set.insert(category.bogus);
            for x in category.kinds {
                set.insert(*x);
            }
        }
        Matcher::AnyNode => set.any_node = true,
        Matcher::AnyElement => set.any_element = true,
        Matcher::Choice(c) => {
            for x in c {
                set.extend(first(schema, *x));
            }
        }
    }
    set
}
fn nullable<K: Copy + Eq, C: Copy + Eq>(matcher: Matcher<K, C>) -> bool {
    match matcher {
        Matcher::Choice(c) => c.iter().any(|x| nullable(*x)),
        _ => false,
    }
}
fn overlap<K: Copy + Eq, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    a: Matcher<K, C>,
    b: Matcher<K, C>,
) -> bool {
    overlap_sets(schema, &first(schema, a), &first(schema, b))
}
fn overlap_sets<K: Copy + Eq, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    a: &First<K>,
    b: &First<K>,
) -> bool {
    if (a.any_element && !b.empty())
        || (b.any_element && !a.empty())
        || a.kinds.iter().any(|k| b.kinds.contains(k))
        || a.kinds
            .iter()
            .any(|kind| b.contextual.iter().any(|(other, _)| other == kind))
        || b.kinds
            .iter()
            .any(|kind| a.contextual.iter().any(|(other, _)| other == kind))
        || a.contextual.iter().any(|item| b.contextual.contains(item))
    {
        return true;
    }
    let is_node = |k: K| schema.nodes.iter().any(|n| n.kind == k);
    (a.any_node && (b.any_node || b.kinds.iter().copied().any(is_node)))
        || (b.any_node && a.kinds.iter().copied().any(is_node))
}
fn sets_overlap<K: Copy + Eq, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    a: &First<K>,
    b: &First<K>,
) -> bool {
    overlap_sets(schema, a, b)
}
fn following_first<K: Copy + Eq, C: Copy + Eq>(
    schema: &SyntaxSchema<K, C>,
    slots: &[SlotShape<K, C>],
) -> Option<First<K>> {
    let mut set = First::default();
    for slot in slots {
        set.extend(first(schema, slot.matcher));
        if slot_required(*slot) && !nullable(slot.matcher) {
            return Some(set);
        }
    }
    (!set.empty()).then_some(set)
}
