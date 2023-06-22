from subprocess import Popen, PIPE, STDOUT
import json
import math

class SnortColor():
    Empty = 0
    TintBlue = 1
    TintRed = 2
    Blue = 3
    Red = 4

class Game:
    def __init__(self, canonical_form, temperature):
        self.canonical_form = canonical_form
        self.temperature = temperature

    def __repr__(self):
        return 'Game(' + self.canonical_form + ',' + self.temperature + ')'

class Cgt:
    def __init__(self, path):
        self.p = Popen([path], stdout=PIPE, stdin=PIPE, stderr=PIPE)

    def request(self, d):
        j = json.dumps(d)
        bs = str.encode(j + "\n")
        self.p.stdin.write(bs)
        self.p.stdin.flush()
        line = self.p.stdout.read1()
        resp = json.loads(line)
        return Game(resp['canonical_form'], resp['temperature'])

    
    def domineering(self, grid):
        d = {
            "Domineering": {
                "grid": grid,
            },
        }
        return self.request(d)

    def snort(self, adjacency_matrix, vertices = None):
        size = math.isqrt(len(adjacency_matrix))
        vs = [SnortColor.Empty]*size if vertices is None else vertices
        d = {
            "Snort": {
                "vertices": vs,
                "adjacency_matrix": adjacency_matrix,
            },
        }
        return self.request(d)
