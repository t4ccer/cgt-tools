from subprocess import Popen, PIPE, STDOUT
import json

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

    
    def domineering(self, width, height, grid):
        d = {
            "Domineering": {
                "width": width,
                "height": height,
                "grid": grid,
            },
        }
        return self.request(d)

    def snort(self, size, adjacency_matrix):
        d = {
            "Snort": {
                "size": size,
                "adjacency_matrix": adjacency_matrix,
            },
        }
        return self.request(d)
