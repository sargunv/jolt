# Java Parser Review Notes

Review performed after commit
`1734bd0 Tighten Java parser recovery and organization`.

These are the remaining architecture questions that need judgement between real
tradeoffs, not straightforward cleanup.

## Open Judgement Calls

- Grammar decisions still depend on duplicated speculative scanners.

  Many branch decisions are made by lookahead helpers such as
  `starts_method_declaration`, `starts_local_variable_declaration`,
  `starts_cast_expression`, `starts_pattern`, and `skip_type_from`. These
  helpers duplicate parts of the real grammar parser, especially type parsing.

  Keeping this style preserves a simple, fast, predictive recursive-descent
  parser. The cost is drift: any new type syntax, annotation placement, pattern
  form, or generic-close rule has to be updated in both the scanner and the real
  parser. When they disagree, the parser chooses the wrong production before
  recovery can be precise.

  The judgement call is whether to keep paying that local simplicity cost, or
  introduce a shared lookahead cursor/type-scanning primitive that makes grammar
  drift harder but adds parser infrastructure.

- Virtual `>` tokens are produced by mutating the parser token vector.

  When a `>>` or `>>>` token closes nested generic type arguments, the parser
  rewrites the token stream with `remove` and `splice`, turning the shift token
  into multiple synthetic `>` tokens.

  This is pragmatic and keeps tree construction simple today. The cost is that
  token indexing becomes mutable, pathological nested generic code can pay
  repeated vector-shift costs, and the final CST no longer has a clean token
  origin model.

  A clean origin model means every parser token can answer where it came from:
  either an original lexer token, or a virtual slice of an original lexer token.
  That matters for formatter-grade tools because wrappers, diagnostics, source
  maps, and token-preserving edits often need to relate CST tokens back to the
  lexer stream and source bytes.

  The judgement call is whether to keep the current simple mutation until
  formatter wrappers prove they need stronger token identity, or invest now in
  logical parser tokens with origin metadata / pending split state.

- Recovery structure may need a formatter-facing policy.

  Lightweight recovery cleanup is done, but deeper recovery design is still a
  product/API call. Today diagnostics are side-channel events and `ErrorNode`s
  mark malformed syntax that was actually consumed. Missing tokens are
  diagnostics, not tree nodes, and skipped ranges are not represented by a
  richer recovery object.

  The judgement call is how much structure formatter wrappers should rely on:
  keep recovery lightweight and infer from diagnostics plus `ErrorNode`s, or add
  more explicit CST/recovery constructs such as missing-token nodes, wider
  skipped-region nodes, or diagnostic ranges tied directly to recovery nodes.

- Typed wrappers need a public API stance.

  The parser exposes raw CST aliases and `JavaSyntaxKind` today. Formatter code
  can build on that, but then CST shape becomes an implicit public contract.

  The judgement call is whether wrappers should closely mirror raw CST shape, or
  expose formatter-oriented roles such as conditions, selectors, return values,
  import kind, and switch-label items. The second option gives consumers a
  better API but creates a stronger wrapper maintenance layer.

- Java language-level/config gating needs a product decision.

  The parser currently acts like one modern Java grammar. Contextual keywords
  such as `record`, `sealed`, `permits`, `var`, and `yield` are interpreted
  according to that grammar.

  The judgement call is whether Jolt formats one current Java dialect, or
  whether parsing should be configurable for older/release-specific Java source
  levels. Supporting multiple language levels would make contextual keyword and
  feature-gating behavior more correct for mixed projects, but it adds config
  and compatibility surface area.
