# Settings UI consistency audit

Generated: 04/18/2026 23:49:00  
Runtime: 17.3s  
Totals: critical=0 major=84 minor=0 (total=84)

## Summary by page × severity

| Page | Critical | Major | Minor |
|------|---------:|------:|------:|
| about | 0 | 14 | 0 |
| advanced | 0 | 28 | 0 |
| captions | 0 | 28 | 0 |
| post-process | 0 | 14 | 0 |

## Page: about

### major (14)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-0.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-42.png)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-1.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-43.png)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-2.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-44.png)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-3.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-45.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-46.png)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-4.png)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-5.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-47.png)

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
  ![R-005-missing-description](screenshots/about-desktop-1280x800-R-005-missing-description-6.png)

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
  ![R-005-missing-description](screenshots/about-mobile-portrait-390x844-R-005-missing-description-48.png)

## Page: advanced

### major (28)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-14.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-56.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-23.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-65.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-66.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-24.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-25.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-67.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-26.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-68.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-27.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-69.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-15.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-57.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-58.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-16.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-17.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-59.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-18.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-60.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-19.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-61.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-20.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-62.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-21.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-63.png)

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
  ![R-005-missing-description](screenshots/advanced-mobile-portrait-390x844-R-005-missing-description-64.png)

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
  ![R-005-missing-description](screenshots/advanced-desktop-1280x800-R-005-missing-description-22.png)

## Page: captions

### major (28)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-28.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-70.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-37.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-79.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-38.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-80.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-39.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-81.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-82.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-40.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-41.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-83.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-29.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-71.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-30.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-72.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-31.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-73.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-32.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-74.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-75.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-33.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-34.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-76.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-35.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-77.png)

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
  ![R-005-missing-description](screenshots/captions-desktop-1280x800-R-005-missing-description-36.png)

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
  ![R-005-missing-description](screenshots/captions-mobile-portrait-390x844-R-005-missing-description-78.png)

## Page: post-process

### major (14)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-7.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-49.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-50.png)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-8.png)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-9.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-51.png)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-10.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-52.png)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-11.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-53.png)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-12.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-54.png)

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
  ![R-005-missing-description](screenshots/post-process-desktop-1280x800-R-005-missing-description-13.png)

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
  ![R-005-missing-description](screenshots/post-process-mobile-portrait-390x844-R-005-missing-description-55.png)


