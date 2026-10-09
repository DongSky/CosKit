# PhotoCraft → CosKit 完整功能对照

来源：所提供参考项目固定提交的原菜单注册表。以下 627 项全部使用同一套原生 UI、命令与引擎实现；没有替换为空按钮，也不重写其参数和交互。菜单接通和 Photoshop 行为一致是不同指标；上游边界仍以原文档为准。

另有工具栏手势、参数面板、内部命令和自动化接口，不局限于这 627 项，亦由完整源码保留。

## File（51）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| File → New… | `file.new` | 原实现完整保留 |
| File → Open… | `file.open` | 原实现完整保留 |
| File → Open As… | `file.openAs` | 原实现完整保留 |
| File → Close | `file.close` | 原实现完整保留 |
| File → Close All | `file.closeAll` | 原实现完整保留 |
| File → Close Others | `file.closeOthers` | 原实现完整保留 |
| File → Save | `file.save` | 原实现完整保留 |
| File → Save As… | `file.saveAs` | 原实现完整保留 |
| File → Save a Copy… | `file.saveACopy` | 原实现完整保留 |
| File → Revert | `file.revert` | 原实现完整保留 |
| File → Export → Quick Export as PNG | `file.export.quickExportAsPng` | 原实现完整保留 |
| File → Export → Export As… | `file.export.exportAs` | 原实现完整保留 |
| File → Export → Export Preferences… | `file.export.exportPreferences` | 原实现完整保留 |
| File → Export → Save for Web (Legacy)… | `file.export.saveForWebLegacy` | 原实现完整保留 |
| File → Export → Artboards to Files… | `file.export.artboardsToFiles` | 原实现完整保留 |
| File → Export → Artboards to PDF… | `file.export.artboardsToPdf` | 原实现完整保留 |
| File → Export → Layers to Files… | `file.export.layersToFiles` | 原实现完整保留 |
| File → Export → Layer Comps to Files… | `file.export.layerCompsToFiles` | 原实现完整保留 |
| File → Export → Color Lookup Tables… | `file.export.colorLookupTables` | 原实现完整保留 |
| File → Export → Data Sets as Files… | `file.export.dataSetsAsFiles` | 原实现完整保留 |
| File → Export → Paths to Illustrator… | `file.export.pathsToIllustrator` | 原实现完整保留 |
| File → Export → Render Video… | `file.export.renderVideo` | 原实现完整保留 |
| File → Generate → Image Assets | `file.generate.imageAssets` | 原实现完整保留 |
| File → Place Embedded… | `file.placeEmbedded` | 原实现完整保留 |
| File → Place Linked… | `file.placeLinked` | 原实现完整保留 |
| File → Package… | `file.package` | 原实现完整保留 |
| File → Automate → Batch… | `file.automate.batch` | 原实现完整保留 |
| File → Automate → Create Droplet… | `file.automate.createDroplet` | 原实现完整保留 |
| File → Automate → Crop and Straighten Photos | `file.automate.cropAndStraightenPhotos` | 原实现完整保留 |
| File → Automate → Contact Sheet II… | `file.automate.contactSheetII` | 原实现完整保留 |
| File → Automate → Conditional Mode Change… | `file.automate.conditionalModeChange` | 原实现完整保留 |
| File → Automate → Fit Image… | `file.automate.fitImage` | 原实现完整保留 |
| File → Automate → Lens Correction… | `file.automate.lensCorrection` | 原实现完整保留 |
| File → Automate → Merge to HDR Pro… | `file.automate.mergeToHdrPro` | 原实现完整保留 |
| File → Automate → Photomerge… | `file.automate.photomerge` | 原实现完整保留 |
| File → Scripts → Image Processor… | `file.scripts.imageProcessor` | 原实现完整保留 |
| File → Scripts → Delete All Empty Layers | `file.scripts.deleteAllEmptyLayers` | 原实现完整保留 |
| File → Scripts → Flatten All Layer Effects | `file.scripts.flattenAllLayerEffects` | 原实现完整保留 |
| File → Scripts → Flatten All Masks | `file.scripts.flattenAllMasks` | 原实现完整保留 |
| File → Scripts → Script Events Manager… | `file.scripts.scriptEventsManager` | 原实现完整保留 |
| File → Scripts → Load Files into Stack… | `file.scripts.loadFilesIntoStack` | 原实现完整保留 |
| File → Scripts → Statistics… | `file.scripts.statistics` | 原实现完整保留 |
| File → Scripts → Browse… | `file.scripts.browse` | 原实现完整保留 |
| File → Import → Variable Data Sets… | `file.import.variableDataSets` | 原实现完整保留 |
| File → Import → Video Frames to Layers… | `file.import.videoFramesToLayers` | 原实现完整保留 |
| File → Import → Notes… | `file.import.notes` | 原实现完整保留 |
| File → Import → WIA Support… | `file.import.wiaSupport` | 原实现完整保留 |
| File → File Info… | `file.fileInfo` | 原实现完整保留 |
| File → Print… | `file.print` | 原实现完整保留 |
| File → Print One Copy | `file.printOneCopy` | 原实现完整保留 |
| File → Exit | `file.exit` | 原实现完整保留 |

## Edit（75）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Edit → Undo | `edit.undo` | 原实现完整保留 |
| Edit → Redo | `edit.redo` | 原实现完整保留 |
| Edit → Toggle Last State | `edit.toggleLastState` | 原实现完整保留 |
| Edit → Fade… | `edit.fade` | 原实现完整保留 |
| Edit → Cut | `edit.cut` | 原实现完整保留 |
| Edit → Copy | `edit.copy` | 原实现完整保留 |
| Edit → Copy Merged | `edit.copyMerged` | 原实现完整保留 |
| Edit → Paste | `edit.paste` | 原实现完整保留 |
| Edit → Paste Special → Paste in Place | `edit.pasteSpecial.pasteInPlace` | 原实现完整保留 |
| Edit → Paste Special → Paste Into | `edit.pasteSpecial.pasteInto` | 原实现完整保留 |
| Edit → Paste Special → Paste Outside | `edit.pasteSpecial.pasteOutside` | 原实现完整保留 |
| Edit → Clear | `edit.clear` | 原实现完整保留 |
| Edit → Search… | `edit.search` | 原实现完整保留 |
| Edit → Check Spelling… | `edit.checkSpelling` | 原实现完整保留 |
| Edit → Find and Replace Text… | `edit.findAndReplaceText` | 原实现完整保留 |
| Edit → Fill… | `edit.fill` | 原实现完整保留 |
| Edit → Stroke… | `edit.stroke` | 原实现完整保留 |
| Edit → Content-Aware Fill… | `edit.contentAwareFill` | 原实现完整保留 |
| Edit → Content-Aware Scale | `edit.contentAwareScale` | 原实现完整保留 |
| Edit → Puppet Warp | `edit.puppetWarp` | 原实现完整保留 |
| Edit → Perspective Warp | `edit.perspectiveWarp` | 原实现完整保留 |
| Edit → Free Transform | `edit.freeTransform` | 原实现完整保留 |
| Edit → Transform → Again | `edit.transform.again` | 原实现完整保留 |
| Edit → Transform → Scale | `edit.transform.scale` | 原实现完整保留 |
| Edit → Transform → Rotate | `edit.transform.rotate` | 原实现完整保留 |
| Edit → Transform → Skew | `edit.transform.skew` | 原实现完整保留 |
| Edit → Transform → Distort | `edit.transform.distort` | 原实现完整保留 |
| Edit → Transform → Perspective | `edit.transform.perspective` | 原实现完整保留 |
| Edit → Transform → Warp | `edit.transform.warp` | 原实现完整保留 |
| Edit → Transform → Split Warp Horizontally | `edit.transform.splitWarpHorizontally` | 原实现完整保留 |
| Edit → Transform → Split Warp Vertically | `edit.transform.splitWarpVertically` | 原实现完整保留 |
| Edit → Transform → Split Warp Crosswise | `edit.transform.splitWarpCrosswise` | 原实现完整保留 |
| Edit → Transform → Remove Warp Split | `edit.transform.removeWarpSplit` | 原实现完整保留 |
| Edit → Transform → Rotate 180° | `edit.transform.rotate180` | 原实现完整保留 |
| Edit → Transform → Rotate 90° Clockwise | `edit.transform.rotate90Cw` | 原实现完整保留 |
| Edit → Transform → Rotate 90° Counter Clockwise | `edit.transform.rotate90Ccw` | 原实现完整保留 |
| Edit → Transform → Flip Horizontal | `edit.transform.flipHorizontal` | 原实现完整保留 |
| Edit → Transform → Flip Vertical | `edit.transform.flipVertical` | 原实现完整保留 |
| Edit → Auto-Align Layers… | `edit.autoAlignLayers` | 原实现完整保留 |
| Edit → Auto-Blend Layers… | `edit.autoBlendLayers` | 原实现完整保留 |
| Edit → Define Brush Preset… | `edit.defineBrushPreset` | 原实现完整保留 |
| Edit → Define Pattern… | `edit.definePattern` | 原实现完整保留 |
| Edit → Define Custom Shape… | `edit.defineCustomShape` | 原实现完整保留 |
| Edit → Purge → Undo | `edit.purge.undo` | 原实现完整保留 |
| Edit → Purge → Clipboard | `edit.purge.clipboard` | 原实现完整保留 |
| Edit → Purge → Histories | `edit.purge.histories` | 原实现完整保留 |
| Edit → Purge → Video Cache | `edit.purge.videoCache` | 原实现完整保留 |
| Edit → Purge → All | `edit.purge.all` | 原实现完整保留 |
| Edit → Presets → Preset Manager… | `edit.presets.presetManager` | 原实现完整保留 |
| Edit → Presets → Migrate Presets | `edit.presets.migratePresets` | 原实现完整保留 |
| Edit → Presets → Export/Import Presets… | `edit.presets.exportImportPresets` | 原实现完整保留 |
| Edit → Color Settings… | `edit.colorSettings` | 原实现完整保留 |
| Edit → Assign Profile… | `edit.assignProfile` | 原实现完整保留 |
| Edit → Convert to Profile… | `edit.convertToProfile` | 原实现完整保留 |
| Edit → Keyboard Shortcuts… | `edit.keyboardShortcuts` | 原实现完整保留 |
| Edit → Menus… | `edit.menus` | 原实现完整保留 |
| Edit → Toolbar… | `edit.toolbar` | 原实现完整保留 |
| Edit → Preferences → Settings… | `edit.preferences.general` | 原实现完整保留 |
| Edit → Preferences → AI Integrations… | `edit.preferences.integrations` | 原实现完整保留 |
| Edit → Preferences → Interface… | `edit.preferences.interface` | 原实现完整保留 |
| Edit → Preferences → Workspace… | `edit.preferences.workspace` | 原实现完整保留 |
| Edit → Preferences → Tools… | `edit.preferences.tools` | 原实现完整保留 |
| Edit → Preferences → History Log… | `edit.preferences.historyLog` | 原实现完整保留 |
| Edit → Preferences → File Handling… | `edit.preferences.fileHandling` | 原实现完整保留 |
| Edit → Preferences → Export… | `edit.preferences.export` | 原实现完整保留 |
| Edit → Preferences → Performance… | `edit.preferences.performance` | 原实现完整保留 |
| Edit → Preferences → Scratch Disks… | `edit.preferences.scratchDisks` | 原实现完整保留 |
| Edit → Preferences → Cursors… | `edit.preferences.cursors` | 原实现完整保留 |
| Edit → Preferences → Transparency & Gamut… | `edit.preferences.transparencyAndGamut` | 原实现完整保留 |
| Edit → Preferences → Units & Rulers… | `edit.preferences.unitsAndRulers` | 原实现完整保留 |
| Edit → Preferences → Guides, Grid & Slices… | `edit.preferences.guidesGridAndSlices` | 原实现完整保留 |
| Edit → Preferences → Plug-ins… | `edit.preferences.plugIns` | 原实现完整保留 |
| Edit → Preferences → Type… | `edit.preferences.type` | 原实现完整保留 |
| Edit → Preferences → Enhanced Controls… | `edit.preferences.enhancedControls` | 原实现完整保留 |
| Edit → Preferences → Raw Defaults… | `edit.preferences.rawDefaults` | 原实现完整保留 |

## Image（61）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Image → Mode → Bitmap | `image.mode.bitmap` | 原实现完整保留 |
| Image → Mode → Grayscale | `image.mode.grayscale` | 原实现完整保留 |
| Image → Mode → Duotone | `image.mode.duotone` | 原实现完整保留 |
| Image → Mode → Indexed Color | `image.mode.indexedColor` | 原实现完整保留 |
| Image → Mode → RGB Color | `image.mode.rgb` | 原实现完整保留 |
| Image → Mode → CMYK Color | `image.mode.cmyk` | 原实现完整保留 |
| Image → Mode → Lab Color | `image.mode.lab` | 原实现完整保留 |
| Image → Mode → Multichannel | `image.mode.multichannel` | 原实现完整保留 |
| Image → Mode → 8 Bits/Channel | `image.mode.bits8` | 原实现完整保留 |
| Image → Mode → 16 Bits/Channel | `image.mode.bits16` | 原实现完整保留 |
| Image → Mode → 32 Bits/Channel | `image.mode.bits32` | 原实现完整保留 |
| Image → Mode → Color Table… | `image.mode.colorTable` | 原实现完整保留 |
| Image → Adjustments → Brightness/Contrast… | `image.adjustments.brightnessContrast` | 原实现完整保留 |
| Image → Adjustments → Levels… | `image.adjustments.levels` | 原实现完整保留 |
| Image → Adjustments → Curves… | `image.adjustments.curves` | 原实现完整保留 |
| Image → Adjustments → Exposure… | `image.adjustments.exposure` | 原实现完整保留 |
| Image → Adjustments → Vibrance… | `image.adjustments.vibrance` | 原实现完整保留 |
| Image → Adjustments → Hue/Saturation… | `image.adjustments.hueSaturation` | 原实现完整保留 |
| Image → Adjustments → Color Balance… | `image.adjustments.colorBalance` | 原实现完整保留 |
| Image → Adjustments → Black & White… | `image.adjustments.blackWhite` | 原实现完整保留 |
| Image → Adjustments → Photo Filter… | `image.adjustments.photoFilter` | 原实现完整保留 |
| Image → Adjustments → Channel Mixer… | `image.adjustments.channelMixer` | 原实现完整保留 |
| Image → Adjustments → Color Lookup… | `image.adjustments.colorLookup` | 原实现完整保留 |
| Image → Adjustments → Invert | `image.adjustments.invert` | 原实现完整保留 |
| Image → Adjustments → Posterize… | `image.adjustments.posterize` | 原实现完整保留 |
| Image → Adjustments → Threshold… | `image.adjustments.threshold` | 原实现完整保留 |
| Image → Adjustments → Gradient Map… | `image.adjustments.gradientMap` | 原实现完整保留 |
| Image → Adjustments → Selective Color… | `image.adjustments.selectiveColor` | 原实现完整保留 |
| Image → Adjustments → Shadows/Highlights… | `image.adjustments.shadowsHighlights` | 原实现完整保留 |
| Image → Adjustments → HDR Toning… | `image.adjustments.hdrToning` | 原实现完整保留 |
| Image → Adjustments → Desaturate | `image.adjustments.desaturate` | 原实现完整保留 |
| Image → Adjustments → Match Color… | `image.adjustments.matchColor` | 原实现完整保留 |
| Image → Adjustments → Replace Color… | `image.adjustments.replaceColor` | 原实现完整保留 |
| Image → Adjustments → Equalize | `image.adjustments.equalize` | 原实现完整保留 |
| Image → Auto Tone | `image.autoTone` | 原实现完整保留 |
| Image → Auto Contrast | `image.autoContrast` | 原实现完整保留 |
| Image → Auto Color | `image.autoColor` | 原实现完整保留 |
| Image → Image Size… | `image.imageSize` | 原实现完整保留 |
| Image → Canvas Size… | `image.canvasSize` | 原实现完整保留 |
| Image → Image Rotation → 180° | `image.imageRotation.180` | 原实现完整保留 |
| Image → Image Rotation → 90° Clockwise | `image.imageRotation.90cw` | 原实现完整保留 |
| Image → Image Rotation → 90° Counter Clockwise | `image.imageRotation.90ccw` | 原实现完整保留 |
| Image → Image Rotation → Arbitrary… | `image.rotation.arbitrary` | 原实现完整保留 |
| Image → Image Rotation → Flip Canvas Horizontal | `image.imageRotation.flipCanvasHorizontal` | 原实现完整保留 |
| Image → Image Rotation → Flip Canvas Vertical | `image.imageRotation.flipCanvasVertical` | 原实现完整保留 |
| Image → Crop | `image.crop` | 原实现完整保留 |
| Image → Trim… | `image.trim` | 原实现完整保留 |
| Image → Reveal All | `image.revealAll` | 原实现完整保留 |
| Image → Duplicate… | `image.duplicate` | 原实现完整保留 |
| Image → Apply Image… | `image.applyImage` | 原实现完整保留 |
| Image → Calculations… | `image.calculations` | 原实现完整保留 |
| Image → Variables → Define… | `image.variables.define` | 原实现完整保留 |
| Image → Variables → Data Sets… | `image.variables.dataSets` | 原实现完整保留 |
| Image → Apply Data Set… | `image.applyDataSet` | 原实现完整保留 |
| Image → Trap… | `image.trap` | 原实现完整保留 |
| Image → Analysis → Set Measurement Scale… | `image.analysis.setMeasurementScale` | 原实现完整保留 |
| Image → Analysis → Select Data Points… | `image.analysis.selectDataPoints` | 原实现完整保留 |
| Image → Analysis → Record Measurements | `image.analysis.recordMeasurements` | 原实现完整保留 |
| Image → Analysis → Ruler Tool | `image.analysis.rulerTool` | 原实现完整保留 |
| Image → Analysis → Count Tool | `image.analysis.countTool` | 原实现完整保留 |
| Image → Analysis → Place Scale Marker… | `image.analysis.placeScaleMarker` | 原实现完整保留 |

## Layer（161）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Layer → New → Layer… | `layer.new.layer` | 原实现完整保留 |
| Layer → New → Layer from Background… | `layer.new.layerFromBackground` | 原实现完整保留 |
| Layer → New → Group… | `layer.new.group` | 原实现完整保留 |
| Layer → New → Group from Layers… | `layer.new.groupFromLayers` | 原实现完整保留 |
| Layer → New → Artboard… | `layer.new.artboard` | 原实现完整保留 |
| Layer → New → Artboard from Group… | `layer.new.artboardFromGroup` | 原实现完整保留 |
| Layer → New → Artboard from Layers… | `layer.new.artboardFromLayers` | 原实现完整保留 |
| Layer → New → Frame from Layers | `layer.new.frameFromLayers` | 原实现完整保留 |
| Layer → New → Layer via Copy | `layer.new.layerViaCopy` | 原实现完整保留 |
| Layer → New → Layer via Cut | `layer.new.layerViaCut` | 原实现完整保留 |
| Layer → Duplicate Layer… | `layer.duplicate` | 原实现完整保留 |
| Layer → Delete → Layer | `layer.delete` | 原实现完整保留 |
| Layer → Delete → Hidden Layers | `layer.delete.hiddenLayers` | 原实现完整保留 |
| Layer → Quick Export as PNG | `layer.quickExportAsPng` | 原实现完整保留 |
| Layer → Export As… | `layer.exportAs` | 原实现完整保留 |
| Layer → Rename Layer | `layer.renameLayer` | 原实现完整保留 |
| Layer → Layer Style → Blending Options… | `layer.layerStyle.blendingOptions` | 原实现完整保留 |
| Layer → Layer Style → Bevel & Emboss… | `layer.layerStyle.bevelEmboss` | 原实现完整保留 |
| Layer → Layer Style → Stroke… | `layer.layerStyle.stroke` | 原实现完整保留 |
| Layer → Layer Style → Inner Shadow… | `layer.layerStyle.innerShadow` | 原实现完整保留 |
| Layer → Layer Style → Inner Glow… | `layer.layerStyle.innerGlow` | 原实现完整保留 |
| Layer → Layer Style → Satin… | `layer.layerStyle.satin` | 原实现完整保留 |
| Layer → Layer Style → Color Overlay… | `layer.layerStyle.colorOverlay` | 原实现完整保留 |
| Layer → Layer Style → Gradient Overlay… | `layer.layerStyle.gradientOverlay` | 原实现完整保留 |
| Layer → Layer Style → Pattern Overlay… | `layer.layerStyle.patternOverlay` | 原实现完整保留 |
| Layer → Layer Style → Outer Glow… | `layer.layerStyle.outerGlow` | 原实现完整保留 |
| Layer → Layer Style → Drop Shadow… | `layer.layerStyle.dropShadow` | 原实现完整保留 |
| Layer → Layer Style → Copy Layer Style | `layer.layerStyle.copyLayerStyle` | 原实现完整保留 |
| Layer → Layer Style → Paste Layer Style | `layer.layerStyle.pasteLayerStyle` | 原实现完整保留 |
| Layer → Layer Style → Clear Layer Style | `layer.layerStyle.clear` | 原实现完整保留 |
| Layer → Layer Style → Global Light… | `layer.layerStyle.globalLight` | 原实现完整保留 |
| Layer → Layer Style → Create Layer | `layer.layerStyle.createLayer` | 原实现完整保留 |
| Layer → Layer Style → Hide All Effects | `layer.layerStyle.hideAllEffects` | 原实现完整保留 |
| Layer → Layer Style → Scale Effects… | `layer.layerStyle.scaleEffects` | 原实现完整保留 |
| Layer → Smart Filter → Disable Smart Filters | `layer.smartFilter.disableSmartFilters` | 原实现完整保留 |
| Layer → Smart Filter → Delete Filter Mask | `layer.smartFilter.deleteFilterMask` | 原实现完整保留 |
| Layer → Smart Filter → Disable Filter Mask | `layer.smartFilter.disableFilterMask` | 原实现完整保留 |
| Layer → Smart Filter → Blending Options… | `layer.smartFilter.blendingOptions` | 原实现完整保留 |
| Layer → Smart Filter → Clear Smart Filters | `layer.smartFilter.clearSmartFilters` | 原实现完整保留 |
| Layer → New Fill Layer → Solid Color… | `layer.newFillLayer.solidColor` | 原实现完整保留 |
| Layer → New Fill Layer → Gradient… | `layer.newFillLayer.gradient` | 原实现完整保留 |
| Layer → New Fill Layer → Pattern… | `layer.newFillLayer.pattern` | 原实现完整保留 |
| Layer → New Adjustment Layer → Brightness/Contrast… | `layer.newAdjustmentLayer.brightnessContrast` | 原实现完整保留 |
| Layer → New Adjustment Layer → Levels… | `layer.newAdjustmentLayer.levels` | 原实现完整保留 |
| Layer → New Adjustment Layer → Curves… | `layer.newAdjustmentLayer.curves` | 原实现完整保留 |
| Layer → New Adjustment Layer → Exposure… | `layer.newAdjustmentLayer.exposure` | 原实现完整保留 |
| Layer → New Adjustment Layer → Vibrance… | `layer.newAdjustmentLayer.vibrance` | 原实现完整保留 |
| Layer → New Adjustment Layer → Hue/Saturation… | `layer.newAdjustmentLayer.hueSaturation` | 原实现完整保留 |
| Layer → New Adjustment Layer → Color Balance… | `layer.newAdjustmentLayer.colorBalance` | 原实现完整保留 |
| Layer → New Adjustment Layer → Black & White… | `layer.newAdjustmentLayer.blackWhite` | 原实现完整保留 |
| Layer → New Adjustment Layer → Photo Filter… | `layer.newAdjustmentLayer.photoFilter` | 原实现完整保留 |
| Layer → New Adjustment Layer → Channel Mixer… | `layer.newAdjustmentLayer.channelMixer` | 原实现完整保留 |
| Layer → New Adjustment Layer → Color Lookup… | `layer.newAdjustmentLayer.colorLookup` | 原实现完整保留 |
| Layer → New Adjustment Layer → Invert… | `layer.newAdjustmentLayer.invert` | 原实现完整保留 |
| Layer → New Adjustment Layer → Posterize… | `layer.newAdjustmentLayer.posterize` | 原实现完整保留 |
| Layer → New Adjustment Layer → Threshold… | `layer.newAdjustmentLayer.threshold` | 原实现完整保留 |
| Layer → New Adjustment Layer → Gradient Map… | `layer.newAdjustmentLayer.gradientMap` | 原实现完整保留 |
| Layer → New Adjustment Layer → Selective Color… | `layer.newAdjustmentLayer.selectiveColor` | 原实现完整保留 |
| Layer → Layer Content Options… | `layer.layerContentOptions` | 原实现完整保留 |
| Layer → Layer Mask → Reveal All | `layer.layerMask.revealAll` | 原实现完整保留 |
| Layer → Layer Mask → Hide All | `layer.layerMask.hideAll` | 原实现完整保留 |
| Layer → Layer Mask → Reveal Selection | `layer.layerMask.revealSelection` | 原实现完整保留 |
| Layer → Layer Mask → Hide Selection | `layer.layerMask.hideSelection` | 原实现完整保留 |
| Layer → Layer Mask → From Transparency | `layer.layerMask.fromTransparency` | 原实现完整保留 |
| Layer → Layer Mask → Delete | `layer.layerMask.delete` | 原实现完整保留 |
| Layer → Layer Mask → Apply | `layer.layerMask.apply` | 原实现完整保留 |
| Layer → Layer Mask → Enable Layer Mask | `layer.layerMask.enabled` | 原实现完整保留 |
| Layer → Layer Mask → Link Layer Mask | `layer.layerMask.linked` | 原实现完整保留 |
| Layer → Mask All Objects | `layer.maskAllObjects` | 原实现完整保留 |
| Layer → Vector Mask → Reveal All | `layer.vectorMask.revealAll` | 原实现完整保留 |
| Layer → Vector Mask → Hide All | `layer.vectorMask.hideAll` | 原实现完整保留 |
| Layer → Vector Mask → Current Path | `layer.vectorMask.currentPath` | 原实现完整保留 |
| Layer → Vector Mask → Delete | `layer.vectorMask.delete` | 原实现完整保留 |
| Layer → Vector Mask → Enable Vector Mask | `layer.vectorMask.enabled` | 原实现完整保留 |
| Layer → Vector Mask → Link Vector Mask | `layer.vectorMask.linked` | 原实现完整保留 |
| Layer → Create Clipping Mask | `layer.createClippingMask` | 原实现完整保留 |
| Layer → Smart Objects → Convert to Smart Object | `layer.smartObjects.convertToSmartObject` | 原实现完整保留 |
| Layer → Smart Objects → New Smart Object via Copy | `layer.smartObjects.newSmartObjectViaCopy` | 原实现完整保留 |
| Layer → Smart Objects → Reveal in Finder | `layer.smartObjects.revealInFinder` | 原实现完整保留 |
| Layer → Smart Objects → Update Modified Content | `layer.smartObjects.updateModifiedContent` | 原实现完整保留 |
| Layer → Smart Objects → Update All Modified Content | `layer.smartObjects.updateAllModifiedContent` | 原实现完整保留 |
| Layer → Smart Objects → Edit Contents | `layer.smartObjects.editContents` | 原实现完整保留 |
| Layer → Smart Objects → Relink to File… | `layer.smartObjects.relinkToFile` | 原实现完整保留 |
| Layer → Smart Objects → Replace Contents… | `layer.smartObjects.replaceContents` | 原实现完整保留 |
| Layer → Smart Objects → Export Contents… | `layer.smartObjects.exportContents` | 原实现完整保留 |
| Layer → Smart Objects → Convert to Linked… | `layer.smartObjects.convertToLinked` | 原实现完整保留 |
| Layer → Smart Objects → Convert to Embedded | `layer.smartObjects.convertToEmbedded` | 原实现完整保留 |
| Layer → Smart Objects → Convert to Layers | `layer.smartObjects.convertToLayers` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Entropy | `layer.smartObjects.stackMode.entropy` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Kurtosis | `layer.smartObjects.stackMode.kurtosis` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Maximum | `layer.smartObjects.stackMode.maximum` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Mean | `layer.smartObjects.stackMode.mean` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Median | `layer.smartObjects.stackMode.median` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Minimum | `layer.smartObjects.stackMode.minimum` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Range | `layer.smartObjects.stackMode.range` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Skewness | `layer.smartObjects.stackMode.skewness` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Standard Deviation | `layer.smartObjects.stackMode.standardDeviation` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Summation | `layer.smartObjects.stackMode.summation` | 原实现完整保留 |
| Layer → Smart Objects → Stack Mode → Variance | `layer.smartObjects.stackMode.variance` | 原实现完整保留 |
| Layer → Smart Objects → Rasterize | `layer.smartObjects.rasterize` | 原实现完整保留 |
| Layer → Smart Objects → Warp | `layer.smartObjects.warp` | 原实现完整保留 |
| Layer → Smart Objects → Perspective Warp | `layer.smartObjects.perspectiveWarp` | 原实现完整保留 |
| Layer → Smart Objects → Puppet Warp | `layer.smartObjects.puppetWarp` | 原实现完整保留 |
| Layer → Video Layers → New Blank Video Layer | `layer.videoLayers.newBlankVideoLayer` | 原实现完整保留 |
| Layer → Video Layers → New Video Layer from File… | `layer.videoLayers.newVideoLayerFromFile` | 原实现完整保留 |
| Layer → Video Layers → Insert Blank Frame | `layer.videoLayers.insertBlankFrame` | 原实现完整保留 |
| Layer → Video Layers → Duplicate Frame | `layer.videoLayers.duplicateFrame` | 原实现完整保留 |
| Layer → Video Layers → Delete Frame | `layer.videoLayers.deleteFrame` | 原实现完整保留 |
| Layer → Video Layers → Replace Footage… | `layer.videoLayers.replaceFootage` | 原实现完整保留 |
| Layer → Video Layers → Interpret Footage… | `layer.videoLayers.interpretFootage` | 原实现完整保留 |
| Layer → Video Layers → Show Altered Video | `layer.videoLayers.showAlteredVideo` | 原实现完整保留 |
| Layer → Video Layers → Restore Frame | `layer.videoLayers.restoreFrame` | 原实现完整保留 |
| Layer → Video Layers → Restore All Frames | `layer.videoLayers.restoreAllFrames` | 原实现完整保留 |
| Layer → Video Layers → Reload Frame | `layer.videoLayers.reloadFrame` | 原实现完整保留 |
| Layer → Video Layers → Rasterize | `layer.videoLayers.rasterize` | 原实现完整保留 |
| Layer → Rasterize → Type | `layer.rasterize.type` | 原实现完整保留 |
| Layer → Rasterize → Shape | `layer.rasterize.shape` | 原实现完整保留 |
| Layer → Rasterize → Fill Content | `layer.rasterize.fillContent` | 原实现完整保留 |
| Layer → Rasterize → Vector Mask | `layer.rasterize.vectorMask` | 原实现完整保留 |
| Layer → Rasterize → Smart Object | `layer.rasterize.smartObject` | 原实现完整保留 |
| Layer → Rasterize → Video | `layer.rasterize.video` | 原实现完整保留 |
| Layer → Rasterize → Layer | `layer.rasterize.layer` | 原实现完整保留 |
| Layer → Rasterize → All Layers | `layer.rasterize.allLayers` | 原实现完整保留 |
| Layer → New Layer Based Slice | `layer.newLayerBasedSlice` | 原实现完整保留 |
| Layer → Group Layers | `layer.groupLayers` | 原实现完整保留 |
| Layer → Ungroup Layers | `layer.ungroupLayers` | 原实现完整保留 |
| Layer → Hide Layers | `layer.hideLayers` | 原实现完整保留 |
| Layer → Arrange → Bring to Front | `layer.arrange.bringToFront` | 原实现完整保留 |
| Layer → Arrange → Bring Forward | `layer.arrange.bringForward` | 原实现完整保留 |
| Layer → Arrange → Send Backward | `layer.arrange.sendBackward` | 原实现完整保留 |
| Layer → Arrange → Send to Back | `layer.arrange.sendToBack` | 原实现完整保留 |
| Layer → Arrange → Reverse | `layer.arrange.reverse` | 原实现完整保留 |
| Layer → Combine Shapes → Unite Shapes | `layer.combineShapes.unite` | 原实现完整保留 |
| Layer → Combine Shapes → Subtract Front Shape | `layer.combineShapes.subtractFrontShape` | 原实现完整保留 |
| Layer → Combine Shapes → Intersect Shape Areas | `layer.combineShapes.intersectShapeAreas` | 原实现完整保留 |
| Layer → Combine Shapes → Exclude Overlapping Shapes | `layer.combineShapes.excludeOverlappingShapes` | 原实现完整保留 |
| Layer → Combine Shapes → Merge Shape Components | `layer.combineShapes.mergeShapeComponents` | 原实现完整保留 |
| Layer → Align → Top Edges | `layer.align.topEdges` | 原实现完整保留 |
| Layer → Align → Vertical Centers | `layer.align.verticalCenters` | 原实现完整保留 |
| Layer → Align → Bottom Edges | `layer.align.bottomEdges` | 原实现完整保留 |
| Layer → Align → Left Edges | `layer.align.leftEdges` | 原实现完整保留 |
| Layer → Align → Horizontal Centers | `layer.align.horizontalCenters` | 原实现完整保留 |
| Layer → Align → Right Edges | `layer.align.rightEdges` | 原实现完整保留 |
| Layer → Distribute → Top Edges | `layer.distribute.topEdges` | 原实现完整保留 |
| Layer → Distribute → Vertical Centers | `layer.distribute.verticalCenters` | 原实现完整保留 |
| Layer → Distribute → Bottom Edges | `layer.distribute.bottomEdges` | 原实现完整保留 |
| Layer → Distribute → Left Edges | `layer.distribute.leftEdges` | 原实现完整保留 |
| Layer → Distribute → Horizontal Centers | `layer.distribute.horizontalCenters` | 原实现完整保留 |
| Layer → Distribute → Right Edges | `layer.distribute.rightEdges` | 原实现完整保留 |
| Layer → Distribute → Horizontally | `layer.distribute.horizontally` | 原实现完整保留 |
| Layer → Distribute → Vertically | `layer.distribute.vertically` | 原实现完整保留 |
| Layer → Lock Layers… | `layer.lockLayers` | 原实现完整保留 |
| Layer → Link Layers | `layer.linkLayers` | 原实现完整保留 |
| Layer → Select Linked Layers | `layer.selectLinkedLayers` | 原实现完整保留 |
| Layer → Merge Layers | `layer.mergeLayers` | 原实现完整保留 |
| Layer → Merge Visible | `layer.mergeVisible` | 原实现完整保留 |
| Layer → Flatten Image | `layer.flattenImage` | 原实现完整保留 |
| Layer → Matting → Color Decontaminate… | `layer.matting.colorDecontaminate` | 原实现完整保留 |
| Layer → Matting → Defringe… | `layer.matting.defringe` | 原实现完整保留 |
| Layer → Matting → Remove Black Matte | `layer.matting.removeBlackMatte` | 原实现完整保留 |
| Layer → Matting → Remove White Matte | `layer.matting.removeWhiteMatte` | 原实现完整保留 |

## Type（44）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Type → Panels → Character Panel | `type.panels.character` | 原实现完整保留 |
| Type → Panels → Paragraph Panel | `type.panels.paragraph` | 原实现完整保留 |
| Type → Panels → Glyphs Panel | `type.panels.glyphs` | 原实现完整保留 |
| Type → Panels → Character Styles Panel | `type.panels.characterStyles` | 原实现完整保留 |
| Type → Panels → Paragraph Styles Panel | `type.panels.paragraphStyles` | 原实现完整保留 |
| Type → Anti-Alias → Sharp | `type.antiAlias.sharp` | 原实现完整保留 |
| Type → Anti-Alias → Crisp | `type.antiAlias.crisp` | 原实现完整保留 |
| Type → Anti-Alias → Strong | `type.antiAlias.strong` | 原实现完整保留 |
| Type → Anti-Alias → Smooth | `type.antiAlias.smooth` | 原实现完整保留 |
| Type → Anti-Alias → Windows LCD | `type.antiAlias.windowsLcd` | 原实现完整保留 |
| Type → Anti-Alias → Windows | `type.antiAlias.windows` | 原实现完整保留 |
| Type → Orientation → Horizontal | `type.orientation.horizontal` | 原实现完整保留 |
| Type → Orientation → Vertical | `type.orientation.vertical` | 原实现完整保留 |
| Type → OpenType → Standard Ligatures | `type.openType.standardLigatures` | 原实现完整保留 |
| Type → OpenType → Contextual Alternates | `type.openType.contextualAlternates` | 原实现完整保留 |
| Type → OpenType → Discretionary Ligatures | `type.openType.discretionaryLigatures` | 原实现完整保留 |
| Type → OpenType → Swash | `type.openType.swash` | 原实现完整保留 |
| Type → OpenType → Oldstyle | `type.openType.oldstyle` | 原实现完整保留 |
| Type → OpenType → Stylistic Alternates | `type.openType.stylisticAlternates` | 原实现完整保留 |
| Type → OpenType → Titling Alternates | `type.openType.titlingAlternates` | 原实现完整保留 |
| Type → OpenType → Ornaments | `type.openType.ornaments` | 原实现完整保留 |
| Type → OpenType → Ordinals | `type.openType.ordinals` | 原实现完整保留 |
| Type → OpenType → Fractions | `type.openType.fractions` | 原实现完整保留 |
| Type → Create Work Path | `type.createWorkPath` | 原实现完整保留 |
| Type → Convert to Shape | `type.convertToShape` | 原实现完整保留 |
| Type → Rasterize Type Layer | `type.rasterizeTypeLayer` | 原实现完整保留 |
| Type → Convert to Paragraph Text | `type.convertToParagraphText` | 原实现完整保留 |
| Type → Convert to Point Text | `type.convertToPointText` | 原实现完整保留 |
| Type → Warp Text… | `type.warpText` | 原实现完整保留 |
| Type → Font Preview Size → Small | `type.fontPreviewSize.small` | 原实现完整保留 |
| Type → Font Preview Size → Medium | `type.fontPreviewSize.medium` | 原实现完整保留 |
| Type → Font Preview Size → Large | `type.fontPreviewSize.large` | 原实现完整保留 |
| Type → Font Preview Size → Extra Large | `type.fontPreviewSize.extraLarge` | 原实现完整保留 |
| Type → Font Preview Size → Huge | `type.fontPreviewSize.huge` | 原实现完整保留 |
| Type → Language Options → Default Features | `type.languageOptions.defaultFeatures` | 原实现完整保留 |
| Type → Language Options → East Asian Features | `type.languageOptions.eastAsianFeatures` | 原实现完整保留 |
| Type → Language Options → Middle Eastern Features | `type.languageOptions.middleEasternFeatures` | 原实现完整保留 |
| Type → Language Options → Middle Eastern & South Asian Composer | `type.languageOptions.middleEasternAndSouthAsianComposer` | 原实现完整保留 |
| Type → Update All Text Layers | `type.updateAllTextLayers` | 原实现完整保留 |
| Type → Replace All Missing Fonts | `type.replaceAllMissingFonts` | 原实现完整保留 |
| Type → Resolve Missing Fonts… | `type.resolveMissingFonts` | 原实现完整保留 |
| Type → Paste Lorem Ipsum | `type.pasteLoremIpsum` | 原实现完整保留 |
| Type → Load Default Type Styles | `type.loadDefaultTypeStyles` | 原实现完整保留 |
| Type → Save Default Type Styles | `type.saveDefaultTypeStyles` | 原实现完整保留 |

## Select（25）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Select → All | `select.all` | 原实现完整保留 |
| Select → Deselect | `select.deselect` | 原实现完整保留 |
| Select → Reselect | `select.reselect` | 原实现完整保留 |
| Select → Inverse Selection | `select.inverse` | 原实现完整保留 |
| Select → Convert to Shape | `select.convertToShape` | 原实现完整保留 |
| Select → All Layers | `select.allLayers` | 原实现完整保留 |
| Select → Deselect Layers | `select.deselectLayers` | 原实现完整保留 |
| Select → Find Layers | `select.findLayers` | 原实现完整保留 |
| Select → Isolate Layers | `select.isolateLayers` | 原实现完整保留 |
| Select → Color Range… | `select.colorRange` | 原实现完整保留 |
| Select → Focus Area… | `select.focusArea` | 原实现完整保留 |
| Select → Subject | `select.subject` | 原实现完整保留 |
| Select → Sky | `select.sky` | 原实现完整保留 |
| Select → Select and Mask… | `select.selectAndMask` | 原实现完整保留 |
| Select → Modify → Border… | `select.modify.border` | 原实现完整保留 |
| Select → Modify → Smooth… | `select.modify.smooth` | 原实现完整保留 |
| Select → Modify → Expand… | `select.modify.expand` | 原实现完整保留 |
| Select → Modify → Contract… | `select.modify.contract` | 原实现完整保留 |
| Select → Modify → Feather… | `select.modify.feather` | 原实现完整保留 |
| Select → Grow | `select.grow` | 原实现完整保留 |
| Select → Similar | `select.similar` | 原实现完整保留 |
| Select → Transform Selection | `select.transformSelection` | 原实现完整保留 |
| Select → Edit in Quick Mask Mode | `select.editInQuickMaskMode` | 原实现完整保留 |
| Select → Load Selection… | `select.loadSelection` | 原实现完整保留 |
| Select → Save Selection… | `select.saveSelection` | 原实现完整保留 |

## Filter（75）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Filter → Last Filter | `filter.lastFilter` | 原实现完整保留 |
| Filter → Convert for Smart Filters | `filter.convertForSmartFilters` | 原实现完整保留 |
| Filter → Filter Gallery… | `filter.filterGallery` | 原实现完整保留 |
| Filter → Camera Raw Filter… | `filter.cameraRaw` | 原实现完整保留 |
| Filter → Adaptive Wide Angle… | `filter.adaptiveWideAngle` | 原实现完整保留 |
| Filter → Lens Correction… | `filter.lensCorrection` | 原实现完整保留 |
| Filter → Liquify… | `filter.liquify` | 原实现完整保留 |
| Filter → Vanishing Point… | `filter.vanishingPoint` | 原实现完整保留 |
| Filter → Blur → Average | `filter.blur.average` | 原实现完整保留 |
| Filter → Blur → Blur | `filter.blur.blur` | 原实现完整保留 |
| Filter → Blur → Blur More | `filter.blur.blurMore` | 原实现完整保留 |
| Filter → Blur → Box Blur… | `filter.blur.boxBlur` | 原实现完整保留 |
| Filter → Blur → Gaussian Blur… | `filter.blur.gaussianBlur` | 原实现完整保留 |
| Filter → Blur → Lens Blur… | `filter.blur.lensBlur` | 原实现完整保留 |
| Filter → Blur → Motion Blur… | `filter.blur.motionBlur` | 原实现完整保留 |
| Filter → Blur → Radial Blur… | `filter.blur.radialBlur` | 原实现完整保留 |
| Filter → Blur → Shape Blur… | `filter.blur.shapeBlur` | 原实现完整保留 |
| Filter → Blur → Smart Blur… | `filter.blur.smartBlur` | 原实现完整保留 |
| Filter → Blur → Surface Blur… | `filter.blur.surfaceBlur` | 原实现完整保留 |
| Filter → Blur Gallery → Field Blur… | `filter.blurGallery.fieldBlur` | 原实现完整保留 |
| Filter → Blur Gallery → Iris Blur… | `filter.blurGallery.irisBlur` | 原实现完整保留 |
| Filter → Blur Gallery → Tilt-Shift… | `filter.blurGallery.tiltShift` | 原实现完整保留 |
| Filter → Blur Gallery → Path Blur… | `filter.blurGallery.pathBlur` | 原实现完整保留 |
| Filter → Blur Gallery → Spin Blur… | `filter.blurGallery.spinBlur` | 原实现完整保留 |
| Filter → Distort → Displace… | `filter.distort.displace` | 原实现完整保留 |
| Filter → Distort → Pinch… | `filter.distort.pinch` | 原实现完整保留 |
| Filter → Distort → Polar Coordinates… | `filter.distort.polarCoordinates` | 原实现完整保留 |
| Filter → Distort → Ripple… | `filter.distort.ripple` | 原实现完整保留 |
| Filter → Distort → Shear… | `filter.distort.shear` | 原实现完整保留 |
| Filter → Distort → Spherize… | `filter.distort.spherize` | 原实现完整保留 |
| Filter → Distort → Twirl… | `filter.distort.twirl` | 原实现完整保留 |
| Filter → Distort → Wave… | `filter.distort.wave` | 原实现完整保留 |
| Filter → Distort → ZigZag… | `filter.distort.zigZag` | 原实现完整保留 |
| Filter → Noise → Add Noise… | `filter.noise.addNoise` | 原实现完整保留 |
| Filter → Noise → Despeckle | `filter.noise.despeckle` | 原实现完整保留 |
| Filter → Noise → Dust & Scratches… | `filter.noise.dustAndScratches` | 原实现完整保留 |
| Filter → Noise → Median… | `filter.noise.median` | 原实现完整保留 |
| Filter → Noise → Reduce Noise… | `filter.noise.reduceNoise` | 原实现完整保留 |
| Filter → Pixelate → Color Halftone… | `filter.pixelate.colorHalftone` | 原实现完整保留 |
| Filter → Pixelate → Crystallize… | `filter.pixelate.crystallize` | 原实现完整保留 |
| Filter → Pixelate → Facet | `filter.pixelate.facet` | 原实现完整保留 |
| Filter → Pixelate → Fragment | `filter.pixelate.fragment` | 原实现完整保留 |
| Filter → Pixelate → Mezzotint… | `filter.pixelate.mezzotint` | 原实现完整保留 |
| Filter → Pixelate → Mosaic… | `filter.pixelate.mosaic` | 原实现完整保留 |
| Filter → Pixelate → Pointillize… | `filter.pixelate.pointillize` | 原实现完整保留 |
| Filter → Render → Flame… | `filter.render.flame` | 原实现完整保留 |
| Filter → Render → Picture Frame… | `filter.render.pictureFrame` | 原实现完整保留 |
| Filter → Render → Tree… | `filter.render.tree` | 原实现完整保留 |
| Filter → Render → Clouds | `filter.render.clouds` | 原实现完整保留 |
| Filter → Render → Difference Clouds | `filter.render.differenceClouds` | 原实现完整保留 |
| Filter → Render → Fibers… | `filter.render.fibers` | 原实现完整保留 |
| Filter → Render → Lens Flare… | `filter.render.lensFlare` | 原实现完整保留 |
| Filter → Render → Lighting Effects… | `filter.render.lightingEffects` | 原实现完整保留 |
| Filter → Sharpen → Sharpen | `filter.sharpen.sharpen` | 原实现完整保留 |
| Filter → Sharpen → Sharpen Edges | `filter.sharpen.sharpenEdges` | 原实现完整保留 |
| Filter → Sharpen → Sharpen More | `filter.sharpen.sharpenMore` | 原实现完整保留 |
| Filter → Sharpen → Smart Sharpen… | `filter.sharpen.smartSharpen` | 原实现完整保留 |
| Filter → Sharpen → Unsharp Mask… | `filter.sharpen.unsharpMask` | 原实现完整保留 |
| Filter → Stylize → Diffuse… | `filter.stylize.diffuse` | 原实现完整保留 |
| Filter → Stylize → Emboss… | `filter.stylize.emboss` | 原实现完整保留 |
| Filter → Stylize → Extrude… | `filter.stylize.extrude` | 原实现完整保留 |
| Filter → Stylize → Find Edges | `filter.stylize.findEdges` | 原实现完整保留 |
| Filter → Stylize → Oil Paint… | `filter.stylize.oilPaint` | 原实现完整保留 |
| Filter → Stylize → Solarize | `filter.stylize.solarize` | 原实现完整保留 |
| Filter → Stylize → Tiles… | `filter.stylize.tiles` | 原实现完整保留 |
| Filter → Stylize → Trace Contour… | `filter.stylize.traceContour` | 原实现完整保留 |
| Filter → Stylize → Wind… | `filter.stylize.wind` | 原实现完整保留 |
| Filter → Video → De-Interlace… | `filter.video.deInterlace` | 原实现完整保留 |
| Filter → Video → NTSC Colors | `filter.video.ntscColors` | 原实现完整保留 |
| Filter → Other… → Custom… | `filter.other.custom` | 原实现完整保留 |
| Filter → Other… → HSB/HSL | `filter.other.hsbHsl` | 原实现完整保留 |
| Filter → Other… → High Pass… | `filter.other.highPass` | 原实现完整保留 |
| Filter → Other… → Maximum… | `filter.other.maximum` | 原实现完整保留 |
| Filter → Other… → Minimum… | `filter.other.minimum` | 原实现完整保留 |
| Filter → Other… → Offset… | `filter.other.offset` | 原实现完整保留 |

## View（74）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| View → Proof Setup → Custom… | `view.proofSetup.custom` | 原实现完整保留 |
| View → Proof Setup → Working CMYK | `view.proofSetup.workingCmyk` | 原实现完整保留 |
| View → Proof Setup → Working Cyan Plate | `view.proofSetup.workingCyanPlate` | 原实现完整保留 |
| View → Proof Setup → Working Magenta Plate | `view.proofSetup.workingMagentaPlate` | 原实现完整保留 |
| View → Proof Setup → Working Yellow Plate | `view.proofSetup.workingYellowPlate` | 原实现完整保留 |
| View → Proof Setup → Working Black Plate | `view.proofSetup.workingBlackPlate` | 原实现完整保留 |
| View → Proof Setup → Working CMY Plate | `view.proofSetup.workingCmyPlate` | 原实现完整保留 |
| View → Proof Setup → Legacy Macintosh RGB | `view.proofSetup.legacyMacintoshRgb` | 原实现完整保留 |
| View → Proof Setup → Internet Standard RGB | `view.proofSetup.internetStandardRgb` | 原实现完整保留 |
| View → Proof Setup → Monitor RGB | `view.proofSetup.monitorRgb` | 原实现完整保留 |
| View → Proof Setup → Color Blindness — Protanopia-type | `view.proofSetup.colorBlindnessProtanopia` | 原实现完整保留 |
| View → Proof Setup → Color Blindness — Deuteranopia-type | `view.proofSetup.colorBlindnessDeuteranopia` | 原实现完整保留 |
| View → Proof Colors | `view.proofColors` | 原实现完整保留 |
| View → Gamut Warning | `view.gamutWarning` | 原实现完整保留 |
| View → Pixel Aspect Ratio → Custom Pixel Aspect Ratio… | `view.pixelAspectRatio.custom` | 原实现完整保留 |
| View → Pixel Aspect Ratio → Square | `view.pixelAspectRatio.square` | 原实现完整保留 |
| View → Pixel Aspect Ratio → D1/DV NTSC (0.91) | `view.pixelAspectRatio.d1DvNtsc` | 原实现完整保留 |
| View → Pixel Aspect Ratio → D1/DV PAL (1.09) | `view.pixelAspectRatio.d1DvPal` | 原实现完整保留 |
| View → Pixel Aspect Ratio → D1/DV NTSC Widescreen (1.21) | `view.pixelAspectRatio.d1DvNtscWidescreen` | 原实现完整保留 |
| View → Pixel Aspect Ratio → HDV 1080/DVCPRO HD 720 (1.33) | `view.pixelAspectRatio.hdv1080` | 原实现完整保留 |
| View → Pixel Aspect Ratio → D1/DV PAL Widescreen (1.46) | `view.pixelAspectRatio.d1DvPalWidescreen` | 原实现完整保留 |
| View → Pixel Aspect Ratio → DVCPRO HD 1080/HDV 1080 (1.5) | `view.pixelAspectRatio.dvcproHd1080` | 原实现完整保留 |
| View → Pixel Aspect Ratio → Anamorphic 2:1 (2) | `view.pixelAspectRatio.anamorphic2To1` | 原实现完整保留 |
| View → Pixel Aspect Ratio Correction | `view.pixelAspectRatioCorrection` | 原实现完整保留 |
| View → 32-bit Preview Options… | `view.thirtyTwoBitPreviewOptions` | 原实现完整保留 |
| View → Zoom In | `view.zoomIn` | 原实现完整保留 |
| View → Zoom Out | `view.zoomOut` | 原实现完整保留 |
| View → Fit on Screen | `view.fitOnScreen` | 原实现完整保留 |
| View → Fit Layer(s) on Screen | `view.fitLayersOnScreen` | 原实现完整保留 |
| View → Fit Artboard on Screen | `view.fitArtboardOnScreen` | 原实现完整保留 |
| View → 100% | `view.actualPixels` | 原实现完整保留 |
| View → 200% | `view.twoHundredPercent` | 原实现完整保留 |
| View → Print Size | `view.printSize` | 原实现完整保留 |
| View → Flip Horizontal | `view.flipHorizontal` | 原实现完整保留 |
| View → Screen Mode → Standard Screen Mode | `view.screenMode.standard` | 原实现完整保留 |
| View → Screen Mode → Full Screen Mode With Menu Bar | `view.screenMode.fullScreenWithMenuBar` | 原实现完整保留 |
| View → Screen Mode → Full Screen Mode | `view.screenMode.fullScreen` | 原实现完整保留 |
| View → Extras | `view.extras` | 原实现完整保留 |
| View → Show → Layer Edges | `view.show.layerEdges` | 原实现完整保留 |
| View → Show → Selection Edges | `view.show.selectionEdges` | 原实现完整保留 |
| View → Show → Target Path | `view.show.targetPath` | 原实现完整保留 |
| View → Show → Grid | `view.show.grid` | 原实现完整保留 |
| View → Show → Guides | `view.show.guides` | 原实现完整保留 |
| View → Show → Canvas Guides | `view.show.canvasGuides` | 原实现完整保留 |
| View → Show → Artboard Guides | `view.show.artboardGuides` | 原实现完整保留 |
| View → Show → Count | `view.show.count` | 原实现完整保留 |
| View → Show → Smart Guides | `view.show.smartGuides` | 原实现完整保留 |
| View → Show → Slices | `view.show.slices` | 原实现完整保留 |
| View → Show → Notes | `view.show.notes` | 原实现完整保留 |
| View → Show → Pixel Grid | `view.show.pixelGrid` | 原实现完整保留 |
| View → Show → Brush Preview | `view.show.brushPreview` | 原实现完整保留 |
| View → Show → Mesh | `view.show.mesh` | 原实现完整保留 |
| View → Show → Edit Pins | `view.show.editPins` | 原实现完整保留 |
| View → Show → All | `view.show.all` | 原实现完整保留 |
| View → Show → Show Extras Options… | `view.show.showExtrasOptions` | 原实现完整保留 |
| View → Rulers | `view.rulers` | 原实现完整保留 |
| View → Snap | `view.snap` | 原实现完整保留 |
| View → Snap To → Guides | `view.snapTo.guides` | 原实现完整保留 |
| View → Snap To → Grid | `view.snapTo.grid` | 原实现完整保留 |
| View → Snap To → Layers | `view.snapTo.layers` | 原实现完整保留 |
| View → Snap To → Slices | `view.snapTo.slices` | 原实现完整保留 |
| View → Snap To → Document Bounds | `view.snapTo.documentBounds` | 原实现完整保留 |
| View → Snap To → All | `view.snapTo.all` | 原实现完整保留 |
| View → Lock Guides | `view.lockGuides` | 原实现完整保留 |
| View → Clear Guides | `view.clearGuides` | 原实现完整保留 |
| View → Clear Selected Artboard Guides | `view.clearSelectedArtboardGuides` | 原实现完整保留 |
| View → Clear Canvas Guides | `view.clearCanvasGuides` | 原实现完整保留 |
| View → New Guide… | `view.newGuide` | 原实现完整保留 |
| View → New Guide Layout… | `view.newGuideLayout` | 原实现完整保留 |
| View → New Guides From Shape | `view.newGuidesFromShape` | 原实现完整保留 |
| View → Lock Slices | `view.lockSlices` | 原实现完整保留 |
| View → Clear Slices | `view.clearSlices` | 原实现完整保留 |
| View → Pattern Preview | `view.patternPreview` | 原实现完整保留 |
| View → Pixel Art Preview | `view.pixelArtPreview` | 原实现完整保留 |

## Window（61）

| 菜单路径 | 原命令 ID | CosKit 实现 |
|---|---|---|
| Window → Arrange → Tile All Vertically | `window.arrange.tileAllVertically` | 原实现完整保留 |
| Window → Arrange → Tile All Horizontally | `window.arrange.tileAllHorizontally` | 原实现完整保留 |
| Window → Arrange → 2-up Vertical | `window.arrange.twoUpVertical` | 原实现完整保留 |
| Window → Arrange → 2-up Horizontal | `window.arrange.twoUpHorizontal` | 原实现完整保留 |
| Window → Arrange → 3-up Vertical | `window.arrange.threeUpVertical` | 原实现完整保留 |
| Window → Arrange → 3-up Horizontal | `window.arrange.threeUpHorizontal` | 原实现完整保留 |
| Window → Arrange → 3-up Stacked | `window.arrange.threeUpStacked` | 原实现完整保留 |
| Window → Arrange → 4-up | `window.arrange.fourUp` | 原实现完整保留 |
| Window → Arrange → 6-up | `window.arrange.sixUp` | 原实现完整保留 |
| Window → Arrange → Consolidate All to Tabs | `window.arrange.consolidateAllToTabs` | 原实现完整保留 |
| Window → Arrange → Cascade | `window.arrange.cascade` | 原实现完整保留 |
| Window → Arrange → Tile | `window.arrange.tile` | 原实现完整保留 |
| Window → Arrange → Float in Window | `window.arrange.floatInWindow` | 原实现完整保留 |
| Window → Arrange → Float All in Windows | `window.arrange.floatAllInWindows` | 原实现完整保留 |
| Window → Arrange → New Window for Document | `window.arrange.newWindowForDocument` | 原实现完整保留 |
| Window → Arrange → Match Zoom | `window.arrange.matchZoom` | 原实现完整保留 |
| Window → Arrange → Match Location | `window.arrange.matchLocation` | 原实现完整保留 |
| Window → Arrange → Match Rotation | `window.arrange.matchRotation` | 原实现完整保留 |
| Window → Arrange → Match All | `window.arrange.matchAll` | 原实现完整保留 |
| Window → Workspace → Essentials | `window.workspace.essentials` | 原实现完整保留 |
| Window → Workspace → Photography | `window.workspace.photography` | 原实现完整保留 |
| Window → Workspace → Painting | `window.workspace.painting` | 原实现完整保留 |
| Window → Workspace → Pixel Art | `window.workspace.pixelArt` | 原实现完整保留 |
| Window → Workspace → Graphic and Web | `window.workspace.graphicAndWeb` | 原实现完整保留 |
| Window → Workspace → Motion | `window.workspace.motion` | 原实现完整保留 |
| Window → Workspace → Reset Workspace | `window.workspace.resetWorkspace` | 原实现完整保留 |
| Window → Workspace → New Workspace… | `window.workspace.newWorkspace` | 原实现完整保留 |
| Window → Workspace → Delete Workspace… | `window.workspace.deleteWorkspace` | 原实现完整保留 |
| Window → Workspace → Lock Workspace | `window.workspace.lockWorkspace` | 原实现完整保留 |
| Window → Actions | `window.panel.actions` | 原实现完整保留 |
| Window → Adjustments | `window.panel.adjustments` | 原实现完整保留 |
| Window → Brush Settings | `window.panel.brushSettings` | 原实现完整保留 |
| Window → Brushes | `window.panel.brushes` | 原实现完整保留 |
| Window → Channels | `window.panel.channels` | 原实现完整保留 |
| Window → Character | `window.panel.character` | 原实现完整保留 |
| Window → Character Styles | `window.panel.characterStyles` | 原实现完整保留 |
| Window → Clone Source | `window.panel.cloneSource` | 原实现完整保留 |
| Window → Color | `window.panel.color` | 原实现完整保留 |
| Window → Glyphs | `window.panel.glyphs` | 原实现完整保留 |
| Window → Gradients | `window.panel.gradients` | 原实现完整保留 |
| Window → Histogram | `window.panel.histogram` | 原实现完整保留 |
| Window → History | `window.panel.history` | 原实现完整保留 |
| Window → Info | `window.panel.info` | 原实现完整保留 |
| Window → Layer Comps | `window.panel.layerComps` | 原实现完整保留 |
| Window → Layers | `window.panel.layers` | 原实现完整保留 |
| Window → Measurement Log | `window.panel.measurementLog` | 原实现完整保留 |
| Window → Modifier Keys | `window.panel.modifierKeys` | 原实现完整保留 |
| Window → Navigator | `window.panel.navigator` | 原实现完整保留 |
| Window → Notes | `window.panel.notes` | 原实现完整保留 |
| Window → Paragraph | `window.panel.paragraph` | 原实现完整保留 |
| Window → Paragraph Styles | `window.panel.paragraphStyles` | 原实现完整保留 |
| Window → Paths | `window.panel.paths` | 原实现完整保留 |
| Window → Patterns | `window.panel.patterns` | 原实现完整保留 |
| Window → Properties | `window.panel.properties` | 原实现完整保留 |
| Window → Shapes | `window.panel.shapes` | 原实现完整保留 |
| Window → Styles | `window.panel.styles` | 原实现完整保留 |
| Window → Swatches | `window.panel.swatches` | 原实现完整保留 |
| Window → Timeline | `window.panel.timeline` | 原实现完整保留 |
| Window → Tool Presets | `window.panel.toolPresets` | 原实现完整保留 |
| Window → Options | `window.panel.options` | 原实现完整保留 |
| Window → Tools | `window.panel.tools` | 原实现完整保留 |

