# meridian-ui-scene2d

Turns a laid-out `meridian-ui-domain` element tree into a
`meridian-rendering-domain` `Scene2d` ([ADR 021](../../docs/adr/021-ui-extraction-and-extensibility.md)):
one tinted sprite per visible background, one sprite per glyph, and the clip
spans of scroll viewers (`extract_clipped`, drawn with
`batch_scene2d_clipped`); images next. No GPU and no windowing here.
