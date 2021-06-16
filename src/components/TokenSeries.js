import React, { useEffect, useState } from 'react';
import Menu from 'url:../img/menu-small.svg';

export const TokenSeries = (props) => {

    const { token, account, views } = props
    const { sales } = views
    const { token_type } = token

    return <>
        <div className="content">
            <div className="bids-type">
                <div>
                    <div className="label">Low</div>
                    <div className="amount">$200</div>
                </div>
                <div>
                    <div className="label">High</div>
                    <div className="amount">$1,890</div>
                </div>
            </div>
            <div className="description">
                <h4>36/47 editions available</h4>
                <p>Each edition corresponds to a specific year in the hip-hop industry and is an independent NFT with their own bids. Thus, Edition #1 corresponds to the period from 1973 to 1974, Edition #2 to the period from 1974 to 1975, and so on till 2021.</p>
            </div>
            <div className="ending">
                <p>Auction ends in:</p>
                <h2>42 : 13 : 05</h2>
            </div>
            <div className="select"
                onClick={() => history.push('/edition/' + token_type)}
            >
                <div>Select Edition</div>
                <img src={Menu} />
            </div>
        </div>
    </>
};

