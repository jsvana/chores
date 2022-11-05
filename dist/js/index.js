"use strict";
const fetchChores = async () => {
    let response = await fetch("/api/chores");
    return (await response.json()).chores;
};
const createChoreCard = (chore) => {
    let card = document.createElement("div");
    card.classList.add("card");
    let cardDivider = document.createElement("div");
    cardDivider.classList.add("card-divider");
    let title = document.createElement("h2");
    if (chore.status === "completed") {
        let struckOut = document.createElement("s");
        struckOut.textContent = chore.title;
        title.appendChild(struckOut);
    }
    else {
        title.textContent = chore.title;
        if (chore.overdue) {
            let overdue = document.createElement("span");
            overdue.classList.add("label");
            overdue.classList.add("warning");
            overdue.textContent = "OVERDUE";
            overdue.style.fontSize = "1em";
            title.appendChild(overdue);
        }
    }
    cardDivider.appendChild(title);
    card.appendChild(cardDivider);
    let cardContent = document.createElement("div");
    let description = document.createElement("p");
    description.textContent = chore.description;
    cardContent.appendChild(description);
    let expectedDate = new Date(chore.expected_completion_time * 1000);
    let expectedTime = document.createElement("p");
    expectedTime.textContent = expectedDate.toLocaleString();
    cardContent.appendChild(expectedTime);
    card.appendChild(cardContent);
    return card;
};
const removeAllChildren = (parent) => {
    while (parent.firstChild) {
        parent.removeChild(parent.firstChild);
    }
};
const setChores = async () => {
    let chores = await fetchChores();
    let choresNode = document.querySelector("#chores");
    if (choresNode === null) {
        return;
    }
    removeAllChildren(choresNode);
    for (let chore of chores) {
        choresNode.appendChild(createChoreCard(chore));
    }
};
const updateChores = async () => {
    console.log("call");
    await setChores();
    setTimeout(updateChores, 10000);
};
updateChores();
