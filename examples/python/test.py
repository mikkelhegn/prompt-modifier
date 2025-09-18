from app import before, after

def test_prompts():
    assert before("Hello!") == "Hello! Make sure your reply is appropriate with children under the age of 10."
    assert after("Hello!" == "Hello!")

if __name__ == "__main__":
    test_prompts()
    print("Everything passed")

