# Settings UI consistency audit

Generated: 04/18/2026 23:43:02  
Runtime: 20.7s  
Totals: critical=0 major=130 minor=0 (total=130)

## Summary by page × severity

| Page | Critical | Major | Minor |
|------|---------:|------:|------:|
| about | 0 | 18 | 0 |
| advanced | 0 | 48 | 0 |
| captions | 0 | 48 | 0 |
| post-process | 0 | 16 | 0 |

## Page: about

### major (18)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/about-desktop-1280x800-R-003-layout-0.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/about-desktop-1280x800-R-003-layout-1.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/about-desktop-1280x800-R-003-layout-2.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/about-desktop-1280x800-R-003-layout-3.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-4.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-72.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-5.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-73.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-74.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-6.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-7.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-75.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-8.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-76.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-9.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-77.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-78.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/about/AboutSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-10.png)

## Page: advanced

### major (48)

- rule: `R-003-export-two-column` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(14)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-export-two-column](screenshots/advanced-desktop-1280x800-R-003-export-two-column-20.png)

- rule: `R-003-export-two-column` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(15)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-export-two-column](screenshots/advanced-desktop-1280x800-R-003-export-two-column-21.png)

- rule: `R-003-export-two-column` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(16)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-export-two-column](screenshots/advanced-desktop-1280x800-R-003-export-two-column-22.png)

- rule: `R-003-export-two-column` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(17)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-export-two-column](screenshots/advanced-desktop-1280x800-R-003-export-two-column-23.png)

- rule: `R-005-color-light-grey-on-white` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"settings-outer\"] button",
  "expected": {
    "readableContrast": true
  },
  "actual": {
    "text": "Run preflight",
    "colorHSL": [
      0,
      0,
      1
    ],
    "bgHSL": [
      0,
      0,
      0.984313725490196
    ]
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-color-light-grey-on-white](screenshots/advanced-desktop-1280x800-R-005-color-light-grey-on-white-45.png)

- rule: `R-005-color-light-grey-on-white` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"settings-outer\"] button",
  "expected": {
    "readableContrast": true
  },
  "actual": {
    "text": "Run preflight",
    "colorHSL": [
      0,
      0,
      1
    ],
    "bgHSL": [
      0,
      0,
      0.984313725490196
    ]
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-color-light-grey-on-white](screenshots/advanced-mobile-portrait-390x844-R-005-color-light-grey-on-white-107.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-24.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-86.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-36.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-98.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-38.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-100.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-40.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-102.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-104.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-42.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(18)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-44.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(18)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-106.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-25.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-87.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-26.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-88.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-27.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-89.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-29.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-91.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-31.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-93.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-33.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-95.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(8)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-96.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(8)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-34.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(9)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-35.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(9)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-97.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-37.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-99.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-39.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-101.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-41.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-103.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-43.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-105.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-28.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-90.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-92.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-30.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-mobile-portrait-390x844-R-005-range-editable-94.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/advanced-desktop-1280x800-R-005-range-editable-32.png)

## Page: captions

### major (48)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(14)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/captions-desktop-1280x800-R-003-layout-46.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(15)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/captions-desktop-1280x800-R-003-layout-47.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(16)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/captions-desktop-1280x800-R-003-layout-48.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(17)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/captions-desktop-1280x800-R-003-layout-49.png)

- rule: `R-005-color-light-grey-on-white` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"settings-outer\"] button",
  "expected": {
    "readableContrast": true
  },
  "actual": {
    "text": "Run preflight",
    "colorHSL": [
      0,
      0,
      1
    ],
    "bgHSL": [
      0,
      0,
      0.984313725490196
    ]
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-color-light-grey-on-white](screenshots/captions-desktop-1280x800-R-005-color-light-grey-on-white-71.png)

- rule: `R-005-color-light-grey-on-white` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"settings-outer\"] button",
  "expected": {
    "readableContrast": true
  },
  "actual": {
    "text": "Run preflight",
    "colorHSL": [
      0,
      0,
      1
    ],
    "bgHSL": [
      0,
      0,
      0.984313725490196
    ]
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-color-light-grey-on-white](screenshots/captions-mobile-portrait-390x844-R-005-color-light-grey-on-white-129.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-50.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-108.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-62.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-120.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-64.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-122.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-66.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-124.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-126.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-68.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(18)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-70.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(18)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-128.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-51.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-109.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-52.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-110.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-53.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-111.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-55.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-113.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-57.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-115.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-59.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-117.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(8)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-118.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(8)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-60.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(9)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-61.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(9)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-119.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-63.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(10) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-121.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-65.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(11) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-123.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-67.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(12) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-125.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-69.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(13) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-127.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-54.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-112.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-56.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-114.png)

- rule: `R-005-range-editable` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-mobile-portrait-390x844-R-005-range-editable-116.png)

- rule: `R-005-range-editable` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6) input[type=\"range\"]",
  "expected": {
    "hasNumberInputOrContenteditable": true
  },
  "actual": {
    "numberInputs": 0,
    "contenteditable": 0
  },
  "fileHint": "src/components/settings/advanced/AdvancedSettings.tsx"
}
  ```
  ![R-005-range-editable](screenshots/captions-desktop-1280x800-R-005-range-editable-58.png)

## Page: post-process

### major (16)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/post-process-desktop-1280x800-R-003-layout-11.png)

- rule: `R-003-layout` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "display": "flex|grid",
    "flexDirection": "row (if flex)",
    "gridTracks": ">=2 (if grid)"
  },
  "actual": {
    "display": "block",
    "flexDirection": "row",
    "gridTemplateColumns": "none"
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-003-layout](screenshots/post-process-desktop-1280x800-R-003-layout-12.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-13.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(1)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-79.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-14.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(2)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-80.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-15.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(3)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-81.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-16.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(4)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-82.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-17.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(5)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-83.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-18.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(6)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-84.png)

- rule: `R-005-missing-description` viewport: `desktop-1280x800`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-19.png)

- rule: `R-005-missing-description` viewport: `mobile-portrait-390x844`
  ```json
{
  "selector": "[data-testid=\"setting-row\"]:nth-of-type(7)",
  "expected": {
    "descriptionPresent": true,
    "descriptionNonEmpty": true
  },
  "actual": {
    "descriptionPresent": false,
    "descriptionText": null
  },
  "fileHint": "src/components/settings/post-processing/PostProcessingSettings.tsx"
}
  ```
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-85.png)


