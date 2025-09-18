def before(prompt: str) -> str:
    my_prompt = " Make sure your reply is appropriate with children under the age of 10."
    return prompt + my_prompt

def after(prompt: str) -> str:
    return prompt
