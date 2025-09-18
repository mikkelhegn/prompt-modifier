export const promptmodification = {
    before(userprompt) {
        return userprompt;
    },

    after(promptresult) {
        return promptresult + "\nThis was JS!";
    }
};

// jco componentize --wit ../../shared/wit/world.wit --world-name promptmodifier --out promptmodifierJS.wasm --disable=all prompt_manipulator.js