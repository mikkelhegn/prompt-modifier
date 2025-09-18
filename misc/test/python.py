import wit_world
class Stringprocessing(wit_world.WitWorld):
    def processstring(self, a) -> str:
        # Capitalize the string
        #b = a.capitalize()

        # Revert the string
        b = a[::-1]
        return b
