from cgt import Cgt, SnortColor

# Path to the `cgt-py-adapter` executable. See README.md how to get it.
cgt = Cgt('./../target/debug/cgt-py-adapter')

# cgt.domineering(WIDTH, HEIGHT, GRID)
print(cgt.domineering(2, 3, "..|.#|#.")) # (1)

# cgt.snort(ADJACENCY_MATRIX)
# If you don't provide second argument, all graph vertices will be empty,
# thus (2) and (3) are the same, for custom initial colors see (4)
# Note that the adjacency matrix must form undirected graph
print(cgt.snort([False, False, False,
                 False, False, True,
                 False, True, False])) # (2)

print(cgt.snort([False, False, False,
                 False, False, True,
                 False, True, False],
                [SnortColor.Empty, SnortColor.Empty, SnortColor.Empty])) # (3)

# cgt.snort(ADJACENCY_MATRIX, VERTICES)
# You can provide the second optional argument to set initial vertices colors other than empty
# Available options are `SnortColor.TintBlue`, `SnortColor.TintRed`, and `SnortColor.Empty`.
print(cgt.snort([False, False, False,
                 False, False, True,
                 False, True, False],
                [SnortColor.TintBlue, SnortColor.Empty, SnortColor.Empty])) # (4)
