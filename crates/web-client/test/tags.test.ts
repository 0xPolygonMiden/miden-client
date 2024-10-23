import { expect } from 'chai';
import { testingPage } from "./mocha.global.setup.mjs";

// ADD_TAG TESTS
// =======================================================================================================

interface AddTagSuccessResult {
    tag: string;
    tags: string[];
}

export const addTag = async (tag: string): Promise<AddTagSuccessResult> => {
    return await testingPage.evaluate(async (tag) => {

        const client = window.client;
        await client.add_tag(tag);
        const tags = await client.list_tags();

        return {
            tag: tag,
            tags: tags
        };
    }, tag);
};

describe("add_tag tests", () => {
    it("adds a tag to the system", async () => {
        const tag = "123";
        const result = await addTag(tag);

        expect(result.tags).to.include(tag);
    });
});

// REMOVE_TAG TESTS
// =======================================================================================================

interface RemoveTagSuccessResult {
    tag: string;
    tags: string[];
}

export const removeTag = async (tag: string): Promise<RemoveTagSuccessResult> => {
    return await testingPage.evaluate(async (tag) => {

        const client = window.client;
        await client.add_tag(tag);
        await client.remove_tag(tag);

        const tags = await client.list_tags();

        return {
            tag: tag,
            tags: tags
        };
    }, tag);
};

describe("remove_tag tests", () => {
    it("removes a tag from the system", async () => {
        const tag = "321";
        const result = await removeTag(tag);

        expect(result.tags).to.not.include(tag);
    });
});
