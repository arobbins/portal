import { test } from "../support/test";
test("probe", async ({ page }) => {
  await page.goto("/styleguide");
  const m = await page.evaluate(() => {
    const box = (el: Element | null) => el!.getBoundingClientRect();
    const forms = [...document.querySelectorAll(".styleguide__section")].find((s) => s.querySelector("h2")?.textContent === "Forms")!;
    const label = forms.querySelector("label.a-form-label")!;
    const input = document.getElementById(label.getAttribute("for")!)!;
    const legend = forms.querySelector(".a-form-legend")!;
    const firstChoice = legend.parentElement!.querySelector(".a-form-choice")!;
    const choices = [...legend.parentElement!.querySelectorAll(".a-form-choice")];
    const groups = [...forms.querySelectorAll(".a-form-group")].slice(0, 3);
    const error = forms.querySelector(".a-form-error")!;
    const errorField = error.previousElementSibling!;
    return {
      labelToInput: Math.round(box(input).top - box(label).bottom),
      legendToFirstChoice: Math.round(box(firstChoice).top - box(legend).bottom),
      choiceToChoice: Math.round(box(choices[1]).top - box(choices[0]).bottom),
      groupToGroup: Math.round(box(groups[1]).top - box(groups[0]).bottom),
      fieldToError: Math.round(box(error).top - box(errorField).bottom),
      errorToButton: Math.round(box(error.nextElementSibling!).top - box(error).bottom),
      labelFont: getComputedStyle(label).fontSize, legendFont: getComputedStyle(legend).fontSize, legendColor: getComputedStyle(legend).color, labelColor: getComputedStyle(label).color,
      duplicateIds: [...document.querySelectorAll("[id]")].map((e) => e.id).filter((id, i, all) => all.indexOf(id) !== i),
    };
  });
  console.log(JSON.stringify(m));
});
