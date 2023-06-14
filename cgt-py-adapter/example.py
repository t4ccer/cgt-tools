from cgt import Cgt, SnortColor
    
cgt = Cgt('./../target/debug/cgt-py-adapter')
print(cgt.domineering(2, 3, "..|.#|#."))
print(cgt.snort([False, False, False,
                 False, False, True,
                 False, True, False]))

print(cgt.snort([False, False, False,
                 False, False, True,
                 False, True, False],
                [SnortColor.Empty, SnortColor.Empty, SnortColor.Empty]))

print(cgt.snort([False, False, False,
                 False, False, True,
                 False, True, False],
                [SnortColor.TintBlue, SnortColor.Empty, SnortColor.Empty]))
