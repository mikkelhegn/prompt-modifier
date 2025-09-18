import promptmodifier.wit_world
class Promptmodification(promptmodifier.wit_world.WitWorld):
    def before(self, userprompt: str) -> str:
        my_prompt = " Make sure your reply is appropriate with children under the age of 10."
        return userprompt + my_prompt

    def after(self, promptresult: str) -> str:
        return promptresult