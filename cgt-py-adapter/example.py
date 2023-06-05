from cgt import Cgt
    
cgt = Cgt('./../target/debug/cgt-py-adapter')
print(cgt.domineering(2, 3, "..|.#|#."))
print(cgt.snort(3, [False, False, False, False, False, False, False, True, False]))
