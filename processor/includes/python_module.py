from before import before
from after import after

import promptmodifier.wit_world
class Promptmodification(promptmodifier.wit_world.WitWorld):
    def before(self, userprompt: str) -> str:
        return before(userprompt)

    def after(self, promptresult: str) -> str:
        return after(promptresult)