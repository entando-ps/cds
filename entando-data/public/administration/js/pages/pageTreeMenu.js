$(function () {
    
    const match = window.location.href.match(/(^.+\/do\/)/ );
    const baseUrl = match ? match[0] : window.location.protocol + '//' + window.location.host;

    const serviceUrl = baseUrl + 'rs/PageModel/frames?code=' + PROPERTY.pagemodel,
            addWidgetUrl = baseUrl + 'rs/Page/joinWidget?code=' + PROPERTY.pagemodel,
            moveWidgetUrl = baseUrl + 'rs/Page/moveWidget?code=' + PROPERTY.pagemodel,
            deleteWidgetUrl = baseUrl + 'rs/Page/deleteWidget?code=' + PROPERTY.pagemodel,
            getPageDetailUrl = baseUrl + 'rs/Page/detail',
            restoreOnlineUrl = baseUrl + 'rs/Page/restoreOnlineConfig',
            setOnlineUrl = baseUrl + 'rs/Page/setOnline',
            setOfflineUrl = baseUrl + 'rs/Page/setOffline',
            configureWidgetUrl = baseUrl + 'Page/editFrame.action',
            apiMappingsUrl = baseUrl + 'rs/Portal/WidgetType/apiMappings',
            widgetTypeDetailUrl = baseUrl + 'rs/Portal/WidgetType/details',
            apiCopyFromWidgetUrl = baseUrl + 'Api/Service/copyFromWidget.action',
            pageModelDetailUrl = baseUrl + 'rs/PageModel/details',
            PAGE_IS_SELECTED = !!PROPERTY.pagemodel,
            labels = {
                "deleteWidget": {
                    "title": null,
                    "confirmButton": null,
                    "cancelButton": null
                }
            };


    const gridSlots = {}, // contains previous slots HTML
        alertService = new EntandoAlert('.alert-container');
    let apiMappings = {}, // contains the API mappings
        pageData = null, // contains page details data
        pageFrames = []; // contains the page frames


    // jQuery selectors
    const $gridContainer = $('.grid-container'),
            $pageCircle = $('#pageTree tr#' + PROPERTY.code + ' .statusField .fa, .page-title-container .fa'),
            $restoreOnlineBtn = $('.restore-online-btn'),
            $publishBtn = $('.publish-btn'),
            $unpublishBtn = $('.unpublish-btn'),
            $pageInfo = $('#page-info'),
            $pageTitleBig = $('.page-title-big'),
            $pageTitleTree = $('#pageTree tr#' + PROPERTY.code + ' .tree-item-page-title');

    /**
     * Restores online configuration of the page
     */
    function restoreOnlineConfig() {
        $.ajax(restoreOnlineUrl, {
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                pageCode: PROPERTY.code
            }),
            success: function (data) {
                if (alertService.showResponseAlerts(data)) {
                    return;
                }
                updatePageStatus(data.page);
                updateOnTheFlyDropdown();
                updateDefaultWidgetBtn();
                initPage();
            }
        });
    }

    /**
     * Publish / unpublish the page
     * @param {boolean} online - true to publish, false to unpublish
     */
    function setPageOnlineStatus(online) {
        $.ajax(online ? setOnlineUrl : setOfflineUrl, {
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                pageCode: PROPERTY.code
            }),
            success: function (data) {
                if (alertService.showResponseAlerts(data)) {
                    return;
                }
                updatePageStatus(data.page);
                updateOnTheFlyDropdown();
                updateDefaultWidgetBtn();
            }
        });
    }




    /**
     * Updates the page status circle color
     */
    function updatePageStatus(newData) {

        pageData = newData || pageData;
        const hasChanges = pageData.changed;

        // updates the yellow/green page circles
        const colorClass = pageData.online ? hasChanges ? 'yellow' : 'green' : 'gray';
        let title = pageData.online ? hasChanges ? 'Online \u2260 Draft' : 'Online' : 'Draft';
        $pageCircle
                .removeClass('green yellow gray')
                .addClass(colorClass)
                .attr('title', title);

        // updates the buttons visibility
        const enablePublish = !(!pageData.online || pageData.online && hasChanges);
        const enableUnpublish = !pageData.online;
        const enableRestoreOnline = !(pageData.online && hasChanges);

        $restoreOnlineBtn.prop('disabled', enableRestoreOnline);
        $publishBtn.prop('disabled', enablePublish);
        $unpublishBtn.prop('disabled', enableUnpublish);


        // diff
        $('.diff-slot').removeClass('diff-slot');
        $('.grid-slot').find('.ghost').remove();
        if (pageData.online) {
            if (pageData.draftWidgets.length === pageData.onlineWidgets.length) {
                for (let i = 0; i < pageData.draftWidgets.length; ++i) {
                    const $gridSlot = $('.grid-slot[data-pos="' + i + '"]'),
                          draftW = pageData.draftWidgets[i],
                          onlineW = pageData.onlineWidgets[i];
                    if (!_.isEqual(draftW, onlineW)) {
                        $gridSlot.addClass('diff-slot');
                    }
                    if (!draftW && onlineW) {
                        $gridSlot.append(createGhostIconBlock(i));
                    }
                }
            }
        }

        // titles
        title = getPageTitle(pageData);
        $pageTitleBig.text(title);
        $pageTitleTree.text(title);

        // online/offline class
        $gridContainer.attr('data-online', _.toString(pageData.online));

    }

    /**
     * Initializes page detail and widget data
     * @param {Object} pageData the current pageData object
     */
    function updatePageDetail(pageData, pageModel) {

        const metadata = pageData.draftMetadata,
                checkElems = {
                    'true': '<span title="Yes" class="icon fa fa-check-square-o"></span>',
                    'false': '<span title="No" class="icon fa fa-square-o"></span>'
                };
        $pageInfo.find('[data-info-pagecode]').text(pageData.code);
        const titles = _.map(metadata.titles, function (title, abbr) {
            return '<span class="monospace">(<abbr title="English">' + abbr + '</abbr>)</span> ' + title
        }).join(', ');
        $pageInfo.find('[data-info-titles]').html(titles);
        $pageInfo.find('[data-info-group]').text(pageData.group);
        $pageInfo.find('[data-info-model]').text(pageModel.descr);
        $pageInfo.find('[data-info-showmenu]').html(checkElems[_.toString(metadata.showable)]);
        $pageInfo.find('[data-info-extratitles]').html(checkElems[_.toString(metadata.useExtraTitles)]);

    }



    function isEmptySlot(slot) {
        return _.isEmpty($(slot).find('.grid-widget'));
    }


    /**
     * Shows a warning instead of a grid
     * @param {string} alertText
     */
    function showGridWarning(alertText) {
        const alert = '<div class="alert alert-warning">' +
                '<span class="pficon pficon-warning-triangle-o"></span>' +
                '<strong>' + alertText + '</strong>' +
                '</div>';
        $gridContainer.html(alert);
    }

    /**
     * Shows a warning instead of a grid
     * @param {string} alertText
     */
    function getMessageText(key, args) {
        let msg = TEXT[key] || '';
        if (_.isArray(args)) {
            for (let i = 0; i < args.length; ++i) {
                msg = msg.replace('{' + i + '}', args[i]);
            }
        }
        return msg;
    }

    
    function findWidgetInfo(widgetCode) {
        if (!pageData) {
            return null;
        }
        return _.find(pageData.draftWidgets, {typeCode: widgetCode});
    }
    
    /**
     * Returns the frame element given the position
     * @param {number} framePos the frame position
     * @returns the frame element given the position
     */
    function getFrameElement(framePos) {
    	return $('[data-pos="' + framePos + '"]');
    }
    
    /**
     * Deletes a widget from the grid
     * @param {number} pos the frame position
     */
    function deleteWidget(framePos) {
        // delete the widget
        $.ajax(deleteWidgetUrl, {
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                pageCode: PROPERTY.code,
                frame: framePos
            }),
            success: function (data) {
                if (alertService.showResponseAlerts(data)) {
                    return;
                }
                setEmptySlot(getFrameElement(framePos));

                // update local draft status
                pageData.draftWidgets[framePos] = null;
                updatePageStatus(data.page);
                updateOnTheFlyDropdown();
                updateDefaultWidgetBtn();
            }
        });
    }
    
    function getWidgetTypeDetail(code) {
        let returnValue;
        $.ajax(widgetTypeDetailUrl, {
            async: false,
            method: 'GET',
            dataType: 'json',
            data: {
                widgetTypeCode: code
            },
            success: function (data) {
                returnValue = data;
            }
        });
        return returnValue;
    }

    /**
     * Creates a grid widget element
     * @param {string} widgetCode
     */
    function createGridWidget(widgetCode) {

        const $widget = $('.widget-square[data-widget-id="' + widgetCode + '"]').first(),
                widgetDescr = $widget.find('.widget-name').text(),
                $widgetIcon = $widget.find('.widget-icon').clone(),
                widgetInfo = findWidgetInfo(widgetCode);
        const html = '<div>' +
                '<div class="slot-name"></div>' +
                '</div>';
        
        function createMenuItem(label) {
            const $menuItem = $('<li role="presentation"><a role="menuitem" tabindex="-1" href="#"></a></li>');
            $menuItem.find('a[role="menuitem"]').text(label);
            return $menuItem;
        }

        const $dropdown = $('<div class="dropdown" />');
        $dropdown.append('<i class="menu-btn fa fa-ellipsis-v dropdown-toggle" type="button"  data-toggle="dropdown"></i>');

        const $dropDownMenu = $('<ul class="dropdown-menu dropdown-menu-right" role="menu" aria-labelledby="dropdownMenu">');
        $dropdown.append($dropDownMenu);

        // create menu items
        const $detailsItem = createMenuItem(TEXT['widgetActions.details']);
        $dropDownMenu.append($detailsItem);
        $detailsItem.click(function (e) {
            window.location = PROPERTY.baseUrl +
                    'do/Portal/WidgetType/viewWidgetUtilizers.action?widgetTypeCode=' + widgetCode
        });
        
        const widgetType = getWidgetTypeDetail(widgetCode);
        if (widgetType && widgetType.typeParameters) {
            const $settingsItem = createMenuItem(TEXT['widgetActions.settings']);
            $dropDownMenu.append($settingsItem);
            $settingsItem.click(function (e) {
                const framePos = +$(e.target).closest('.grid-slot').attr('data-pos');
                window.location = configureWidgetUrl +
                        '?pageCode=' + PROPERTY.code + '&frame=' + framePos;
            });
        }

        const apiWidgetCode = widgetInfo && widgetType && widgetType.logic ? _.get(widgetType, 'parentType.code', '') : widgetCode;
        if (PERMISSION.superuser && apiMappings[apiWidgetCode]) {
            const $apiItem = createMenuItem(TEXT['widgetActions.api']);

            $dropDownMenu.append($apiItem);
            $apiItem.click(function (e) {
                const framePos = +$elem.parent().attr('data-pos');
                window.location = apiCopyFromWidgetUrl +
                        '?pageCode=' + PROPERTY.code +
                        '&framePos=' + framePos +
                        '&resourceName=' + apiMappings[apiWidgetCode].resourceName +
                        '&namespace=' + apiMappings[apiWidgetCode].namespace;
            });
        }
        
        if (widgetInfo && widgetInfo.config && widgetType && widgetType.logic === false) {
            const $newWidgetItem = createMenuItem(TEXT['widgetActions.newWidget']);
            $dropDownMenu.append($newWidgetItem);
            $newWidgetItem.click(function (e) {
                const framePos = +$elem.parent().attr('data-pos');
                window.location = PROPERTY.baseUrl +
                        'do/Portal/WidgetType/copy.action?pageCode=' + PROPERTY.code + '&framePos=' + framePos
            });
        }

        const $deleteItem = createMenuItem(TEXT['widgetActions.delete']);
        $dropDownMenu.append($deleteItem);
        $deleteItem.click(function (e) {

            const framePos = +$elem.parent().attr('data-pos');

            $.confirm({
                title: labels.deleteWidget.title,
                content: getModalBody(widgetCode, framePos),
                buttons: {
                    confirm: {
                        text: labels.deleteWidget.confirmButton,
                        btnClass: 'btn-red',
                        action: function () {
                            deleteWidget(framePos);
                        }
                    },
                    cancel: {
                        text: labels.deleteWidget.cancelButton,
                        action: function () {
                            return;
                        }
                    }
                }
            });

        });


        const $iconTextBlock = $('<div />')
                .addClass('icon-text-block')
                .append($widgetIcon)
                .append('<div class="widget-name">' + widgetDescr + '</div>');

        // widget element
        const $elem = $(html)
                .addClass('grid-widget instance')
                .attr('data-widget-id', widgetCode)
                .append($iconTextBlock)
                .append($dropdown);


        return $elem;
    }


    function getWidgetIcon(widgetCode) {
        return $('.widget-list [data-widget-id="' + widgetCode + '"] .widget-icon').clone();
    }

    function createGhostIconBlock(framePos) {
        const onlineWidget = _.get(pageData, 'onlineWidgets');
        if (!onlineWidget) {
            return $('<div />');
        }
        const widgetInfo = pageData.onlineWidgets[framePos];
        const widgetType = getWidgetTypeDetail(widgetInfo.typeCode);
        const widgetCode = widgetInfo.typeCode;
        const widgetDescr = widgetType.titles[PROPERTY.currentLang || PROPERTY.defaultLang];

        const $iconTextBlock = $('<div />')
                .addClass('icon-text-block ghost')
                .append(getWidgetIcon(widgetCode))
                .append('<div class="widget-name">' + widgetDescr + '</div>');

        return $iconTextBlock;
    }

    /**
     * Sets the slot name in the widget
     */
    function setSlotName($widget, html) {
        $widget.find('.slot-name').html(html);
    }


    /**
     * Populates a slot with the provided widget
     * @param {jQuery} $slot
     * @param {jQuery} $widget
     */
    function populateSlot($slot, $widget) {
        setDraggable($widget, $slot);
        setSlotName($widget, _.unescape($slot.attr('data-description')));
        $slot.html($widget);
    }

    /**
     * Empties a slot
     * @param {jQuery} $slot
     */
    function setEmptySlot($slot) {
        const key = $slot.attr('data-pos');
        $slot.html(gridSlots[key]);
    }

    /**
     * Creates the grid given the frames data
     */
    function updateGridPreview(data) {

        try {
            const gen = new GridGenerator({
                frames: data,
                rowHeight: 80
            });

            const gridHtml = '<div class="grid-preview">' + gen.getHtml() + '</div>';
            $gridContainer.html(gridHtml);

            // init original html map (empty slot)
            $('.grid-slot').each(function (index, el) {
                if ($(el).attr('data-pos') !== 'undefined' ){
                    const pos = $(el).attr('data-pos');
                    gridSlots[pos] = $(el).html();
                }
                else {
                    el.classList.remove('grid-slot');
                }
            });

            // populates the slots
            _.forEach(pageData.draftWidgets, function (widget, index) {
                if (widget) {
                    const $curWidget = createGridWidget(_.get(widget, 'typeCode'));
                    const $slot = $('.grid-slot[data-pos="' + index + '"]');
                    populateSlot($slot, $curWidget);
                }
            });

            $('.grid-slot').droppable({
                accept: function (draggable) {

                    const isFree = isEmptySlot(this) || !isEmptySlot($(draggable).parent()),
                            isWidget = $(draggable).hasClass('widget-square')
                            || $(draggable).hasClass('grid-widget');
                    return isWidget && isFree;
                },
                drop: function (ev, ui) {

                    let $prevSlot = $(ui.draggable).parent(),
                            $curSlot = $(ev.target),
                            $curWidget = $(ui.draggable),
                            curWidgetType = $curWidget.attr('data-widget-id'),
                            curSlotPos = +$curSlot.attr('data-pos');


                    if ($prevSlot.is($curSlot)) {
                        return;
                    } else {
                        // replaces the grid slot html with the old (empty) one
                        const html = gridSlots[+$prevSlot.attr('data-pos')];
                        $prevSlot.append(html);
                    }
                    
                    // it's a widget square
                    if (!$curWidget.hasClass('instance')) {
                        $curWidget = createGridWidget(curWidgetType);

                        // add the widget
                        $.ajax(addWidgetUrl, {
                            method: 'POST',
                            contentType: 'application/json',
                            data: JSON.stringify({
                                pageCode: PROPERTY.code,
                                widgetTypeCode: curWidgetType,
                                frame: curSlotPos
                            }),
                            success: function (data) {
                                if (alertService.showResponseAlerts(data)) {
                                    updatePageStatus(data.page);
                                    updateGridPreview(pageData);
                                    updateOnTheFlyDropdown();
                                    updateDefaultWidgetBtn();
                                    return;
                                }
                                // widget needs configuration
                                if (data.redirectLocation) {
                                    window.location = PROPERTY.baseUrl + data.redirectLocation.replace(/^\//, '');
                                    return;
                                }

                                // no need for configuration
                                populateSlot($curSlot, $curWidget);

                                updatePageStatus(data.page);
                                updateOnTheFlyDropdown();
                                updateDefaultWidgetBtn();
                            }
                        });
                    } else {

                        const prevSlotPos = +$prevSlot.attr('data-pos');

                        // move/swap the widget
                        $.ajax(moveWidgetUrl, {
                            method: 'POST',
                            contentType: 'application/json',
                            data: JSON.stringify({
                                swapWidgetRequest: {
                                    pageCode: PROPERTY.code,
                                    src: prevSlotPos,
                                    dest: curSlotPos
                                }
                            }),
                            success: function (data) {
                                if (alertService.showResponseAlerts(data)) {
                                    updatePageStatus(data.page);
                                    updateGridPreview(pageData);
                                    updateOnTheFlyDropdown();
                                    updateDefaultWidgetBtn();

                                    return;
                                }
                                const $prevWidget = $curSlot.find('.grid-widget');
                                setEmptySlot($prevSlot);
                                setEmptySlot($curSlot);


                                if (!_.isEmpty($prevWidget)) {
                                    const $otherWidget = createGridWidget($prevWidget.attr('data-widget-id'));
                                    populateSlot($prevSlot, $otherWidget);
                                }

                                const $newCurWidget = createGridWidget(curWidgetType);
                                populateSlot($curSlot, $newCurWidget);

                                updatePageStatus(data.page);
                                updateOnTheFlyDropdown();
                                updateDefaultWidgetBtn();
                            }
                        });
                    }

                }
            });

        } catch (e) {
            console.error(e, e.data);
            let alertText;
            switch (e.type) {
                case GridGenerator.ERROR.OVERLAPPING_FRAMES:
                    alertText = getMessageText('error.grid.overlappingFrames',
                            [e.data[0].a.description, e.data[0].b.description]);
                    break;
                case GridGenerator.ERROR.MALFORMED_FRAMES:
                    alertText = getMessageText('error.grid.malformedFrames', [e.data[0].description]);
                    break;
                default:
                    alertText = getMessageText('error.grid.gridError');
                    break;

            }

            showGridWarning(alertText);
        }

    }


    function setDraggable(selector) {
        $(selector).draggable({
            helper: function () {
                const id = $(this).attr('data-widget-id');
                return $('.widget-square[data-widget-id="' + id + '"]').clone();
            },
            appendTo: 'body',
            cursorAt: {left: 30, top: 30},
            revert: 'invalid'
        });
    }


    /**
     * On success, initializes pageData and then calls nextFunc if provided
     * @param {Function} onSuccess a function to be called after the load succeeds
     * @param {Function} onError a function to be called after the load fails
     */
    function loadPageData(onSuccess, onError) {

        // Initializes page detail
        $.ajax(getPageDetailUrl, {
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({
                pageCode: PROPERTY.code
            }),
            success: function (data) {
                if (alertService.showResponseAlerts(data)) {
                    return;
                }
                pageData = data.page;
                _.isFunction(onSuccess) && onSuccess(data);
            },
            error: function (data) {
                _.isFunction(onError) && onError(data);
            }
        });
    }


    /**
     * On success, initializes apiMappings and then calls nextFunc if provided
     * @param {Function} onSuccess a function to be called after the load succeeds
     * @param {Function} onError a function to be called after the load fails
     */
    function loadApiMappings(onSuccess, onError) {
        $.ajax(apiMappingsUrl, {
            method: 'POST',
            success: function (data) {
                if (alertService.showResponseAlerts(data)) {
                    return;
                }
                apiMappings = data;
                _.isFunction(onSuccess) && onSuccess(data);
            },
            error: function (data) {
                _.isFunction(onError) && onError(data);
            }
        });
    }

    /**
     * On success, initializes pageFrames and then calls nextFunc if provided
     * @param {Function} onSuccess a function to be called after the load succeeds
     * @param {Function} onError a function to be called after the load fails
     */
    function loadPageFrames(onSuccess, onError) {
        // Initializes the grid
        $.ajax(serviceUrl, {
            method: 'GET',
            success: function (data) {
                if (alertService.showResponseAlerts(data)) {
                    return;
                }
                pageFrames = data;
                _.isFunction(onSuccess) && onSuccess(data);
            },
            error: function (data) {
                _.isFunction(onError) && onError(data);
            }
        });
    }



    function handleApiError() {
        alertService.addDismissableError('Error fetching data');
    }

    /**
     * Initializes all the page after loading all data
     */
    function initPage() {
        loadPageData(function () {
            loadApiMappings(function () {
                loadPageFrames(function () {
                    $.ajax(pageModelDetailUrl, {
                        method: 'GET',
                        dataType: 'json',
                        data: {
                            code: pageData.draftMetadata.modelCode
                        },
                        success: function (data) {
                            updatePageDetail(pageData, data);
                            updateGridPreview(pageFrames);
                            updatePageStatus(pageData);
                            updateOnTheFlyDropdown();
                            updateDefaultWidgetBtn();
                        }
                    });
                }, handleApiError);
            }, handleApiError);
        }, handleApiError);
    }

    function getPageTitle(pageData) {
        let title = pageData.draftMetadata.titles[PROPERTY.currentLang];
        if (title == null || title == 'undefined') {// TODO Verificare
            title = pageData.draftMetadata.titles[PROPERTY.defaultLang];
        }
        return title;
    }
    
    /**
     * Returns true if the page allows on-the-fly publishing.
     * Needs pageData and pageFrames to be populated
     * @returns true if the page allows on-the-fly publishing
     */
    function isOnTheFly() {
        const mainIndex = _.findIndex(pageFrames, {mainFrame: true});
        if (mainIndex !== -1) {
            const mainWidget = pageData.draftWidgets[mainIndex];
            const widgetType = mainWidget ? getWidgetTypeDetail(mainWidget.typeCode) : null;
            return (mainWidget && widgetType && mainWidget.typeCode === 'content_viewer' && !widgetType.config);
        }
        return false;
    }
    
    /**
     * Updates the on the fly dropdown text
     */
    function updateOnTheFlyDropdown() {
    	const text = isOnTheFly() ? TEXT['label.yes'] : TEXT['label.no'];
    	$('.on-the-fly-dropdown-text').text(text);
    }
    
    /**
     * Returns true if the page has the default widgets applied.
     * Needs pageData and pageFrames to be populated
     * @returns true if the page has the default widgets applied
     */
    function isDefaultWidgetApplied() {
    	const draftWidgets = pageData.draftWidgets;
    	for (let i=0; i<draftWidgets.length; ++i) {
    		const defCode = _.get(pageFrames[i], 'defaultWidget.typeCode');
    		const curCode = _.get(draftWidgets[i], 'typeCode');
    		if (defCode !== curCode) {
    			return false;
    		}
    	}
    	return true;
    }
    
    /**
     * Updates the "apply default widget" button
     */
    function updateDefaultWidgetBtn() {
    	if (isDefaultWidgetApplied()) {
    		$('.defwidgets-btn-wrapper').hide();
    		$('.defwidgets-label').show();
    	} else {
    		$('.defwidgets-btn-wrapper').show();
    		$('.defwidgets-label').hide();
    	}
    }

    function setLabels()
    {
        switch (PROPERTY.currentLang) {
            case 'it':
                labels.deleteWidget.title = 'Conferma';
                labels.deleteWidget.cancelButton = 'annulla';
                labels.deleteWidget.confirmButton = 'conferma';
                break;
            case 'en':
                labels.deleteWidget.title = 'Confirm';
                labels.deleteWidget.cancelButton = 'cancel';
                labels.deleteWidget.confirmButton = 'confirm';
                break;
        }
    }

    function getModalBody(widgetName, positionNumber) {
        switch (PROPERTY.currentLang) {
            case 'it':
                return "Cancella widget '" + widgetName + "' dalla pagina '" + PROPERTY.code + "' in posizione '" + positionNumber + "'?";
            case 'en':
                return "Delete widget '" + widgetName + "' from page '" + PROPERTY.code + "' on position '" + positionNumber + "'?";
        }
    }

    //function getModalButtons

    if (PAGE_IS_SELECTED) {
        setDraggable($('.widget-square'), null);


        initPage();
        setLabels();

        $('.restore-online-btn').click(function () {
            restoreOnlineConfig();
        });
        $('.publish-btn').click(function () {
            setPageOnlineStatus(true);
        });
        $('.unpublish-btn').click(function () {
            setPageOnlineStatus(false);
        });
        
        $('.unset-on-the-fly-btn').click(function () {
            if (isOnTheFly()) {
            	const mainIndex = _.findIndex(pageFrames, { mainFrame: true });
            	if (mainIndex !== -1) {
            		deleteWidget(mainIndex);
            	}
            }
        });
        
    } else {
        $('#page-info, [data-target="#page-info"]').remove();
        $('.restore-online-btn').remove();
    }


});//domready
