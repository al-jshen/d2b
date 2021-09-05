# d2b

Command-line tool for generating bibtex from DOIs and arXiv IDs.

## Examples

arXiv identifiers:

```bash
> d2b 1712.01815
```

DOIs:

```bash
> d2b 10.1145/359327.359336
```

Multiple identifiers:

```bash
> d2b 2105.11572 10.1145/359327.359336
```

Identifiers are queried asynchronously. The resulting bibtex is returned in the order of input.

## Example formats:

- 1111.4246
- arxiv:1111.4246
- https://arxiv.org/abs/1111.4246
- 10.18637/jss.v076.i01
- doi:10.18637/jss.v076.i01
- https://doi.org/10.18637/jss.v076.i01
