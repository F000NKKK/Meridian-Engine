# meridian-ui-scene2d

Turns a laid-out `meridian-ui-domain` element tree into a
`meridian-rendering-domain` `Scene2d` ([ADR 021](../../docs/adr/021-ui-extraction-and-extensibility.md)):
one tinted sprite per visible background today, text and images next. No GPU
and no windowing here.
